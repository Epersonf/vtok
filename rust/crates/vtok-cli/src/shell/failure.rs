//! Test failures reduced to what decides the fix: which test, where, and the assertion. Shape-based, so it works when
//! the command line says nothing (wrappers, `make test`, CI scripts): pytest reports, cargo test panics, unittest.
use super::clean::cap;
use super::clean_run::sum_test_results;

/// `File "/a/b/m.py", line 9, in f` -> `m.py:9`
fn short_frame(l: &str) -> String {
    let Some(rest) = l.strip_prefix("File \"") else { return l.to_string() };
    let Some((path, tail)) = rest.split_once('"') else { return l.to_string() };
    let line = tail.trim_start_matches(", line ").split(',').next().unwrap_or("");
    format!("{}:{}", path.rsplit('/').next().unwrap_or(path), line)
}

/// `____ name ____` headers of a pytest FAILURES section.
fn block_title(t: &str) -> Option<String> {
    (t.len() > 6 && t.starts_with("___") && t.ends_with("___")).then(|| t.trim_matches('_').trim().to_string())
}

fn is_py_location(t: &str) -> bool {
    let Some((f, rest)) = t.split_once(':') else { return false };
    f.ends_with(".py") && rest.split(':').next().map_or(false, |n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
}

/// Digits ignored: parametrized failures with the same assertion shape count as one.
fn shape(b: &[String]) -> String {
    b.iter().skip(1).filter(|l| l.trim_start().starts_with("E ")).map(|l| {
        let mut o = String::new();
        for c in l.chars() { if c.is_ascii_digit() { if !o.ends_with('#') { o.push('#'); } } else { o.push(c); } }
        o
    }).collect::<Vec<_>>().join("|")
}

fn pytest_failures(lines: &[String], start: usize, out: &mut Vec<String>) -> Option<()> {
    // one block per failing test: (name, lines)
    let mut blocks: Vec<(String, Vec<String>)> = vec![];
    for l in &lines[start + 1..] {
        let t = l.trim();
        if t.contains("short test summary info") { break; }
        if let Some(n) = block_title(t) { blocks.push((n.clone(), vec![format!("FAIL {n}")])); }
        else if let Some(b) = blocks.last_mut() {
            if t.starts_with("E ") || t == "E" { b.1.push(format!("  {}", cap(t))); }
            else if is_py_location(t) { b.1.push(format!("  at {}", cap(t))); }
        }
    }
    // parametrized failures share a base name: keep the first, plus any whose assertion has a different shape
    let mut order: Vec<String> = vec![];
    for (n, _) in &blocks { let base = n.split('[').next().unwrap_or(n).to_string(); if !order.contains(&base) { order.push(base); } }
    for base in order {
        let group: Vec<&(String, Vec<String>)> = blocks.iter().filter(|(n, _)| n.split('[').next() == Some(base.as_str())).collect();
        let mut seen: Vec<String> = vec![];
        let mut skipped: Vec<&str> = vec![];
        for (n, b) in &group {
            let sh = shape(b);
            if seen.contains(&sh) || seen.len() >= 3 { skipped.push(n.as_str()); } else { seen.push(sh); out.extend(b.iter().cloned()); }
        }
        if !skipped.is_empty() {
            let ids: Vec<&str> = skipped.iter().map(|n| n.split_once('[').map_or(*n, |x| x.1.trim_end_matches(']'))).collect();
            let list = if ids.len() > 6 { format!("{}, {}, … {}", ids[0], ids[1], ids[ids.len() - 1]) } else { ids.join(", ") };
            out.push(format!("  +{} more failures of {base} with the same assertion ({list})", skipped.len()));
        }
    }
    let tail = lines.iter().rev().find(|l| (l.contains(" passed") || l.contains(" failed")) && l.contains(" in "))?;
    out.push(tail.trim_matches(|c: char| c == '=' || c == ' ').to_string());
    Some(())
}

fn cargo_failures(lines: &[String], out: &mut Vec<String>) -> Option<()> {
    let mut inb = false;
    for l in lines {
        if l.starts_with("---- ") && l.ends_with(" stdout ----") { inb = true; out.push(l.trim_matches('-').trim().to_string()); continue; }
        if l == "failures:" || l.starts_with("test result:") || l.starts_with("error: test failed") { inb = false; continue; }
        if inb && !l.trim().is_empty() && !l.starts_with("note: run with") { out.push(format!("  {}", cap(l.trim()))); }
    }
    out.push(sum_test_results(lines)?);
    Some(())
}

/// unittest: one block per failure -> header, innermost frame, exception line.
fn unittest_failures(lines: &[String], out: &mut Vec<String>) {
    let mut frame: Option<String> = None;
    let mut head: Option<String> = None;
    let flush = |out: &mut Vec<String>, head: &mut Option<String>, frame: &mut Option<String>, exc: Option<&str>| {
        if let Some(h) = head.take() {
            out.push(h);
            if let Some(f) = frame.take() { out.push(format!("  {f}")); }
            if let Some(e) = exc { out.push(format!("  {}", cap(e))); }
        }
    };
    for l in lines {
        if l.starts_with("FAIL: ") || l.starts_with("ERROR: ") { head = Some(l.clone()); frame = None; }
        else if head.is_some() && l.trim_start().starts_with("File \"") { frame = Some(short_frame(l.trim())); }
        else if head.is_some() && !l.starts_with(' ') && !l.starts_with('-') && !l.starts_with('=') && !l.is_empty() && !l.starts_with("Traceback") && l.contains(':') { flush(out, &mut head, &mut frame, Some(l)); }
    }
    flush(out, &mut head, &mut frame, None);
    for l in lines.iter().filter(|l| l.starts_with("Ran ") || l.starts_with("FAILED (") || l == &"OK") { out.push(l.clone()); }
}

pub(super) fn failure_view(lines: &[String]) -> Option<String> {
    let mut out: Vec<String> = vec![];
    if let Some(start) = lines.iter().position(|l| l.contains(" FAILURES ") && l.starts_with('=')) {
        pytest_failures(lines, start, &mut out)?;
    } else if lines.iter().any(|l| l.starts_with("---- ") && l.ends_with(" stdout ----")) {
        cargo_failures(lines, &mut out)?;
    } else if lines.iter().any(|l| l.starts_with("Traceback")) && lines.iter().any(|l| l.starts_with("FAIL: ") || l.starts_with("ERROR: ")) && lines.iter().any(|l| l.starts_with("Ran ")) {
        unittest_failures(lines, &mut out);
    } else { return None; }
    Some(out.join("\n") + "\n")
}
