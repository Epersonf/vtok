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
    if lines.iter().any(|l| l.contains(" FAILURES ") && l.starts_with('=')) {
        return crate::views_lang::pytest_view(lines);
    } else if lines.iter().any(|l| l.starts_with("---- ") && l.ends_with(" stdout ----")) {
        cargo_failures(lines, &mut out)?;
    } else if lines.iter().any(|l| l.starts_with("Traceback")) && lines.iter().any(|l| l.starts_with("FAIL: ") || l.starts_with("ERROR: ")) && lines.iter().any(|l| l.starts_with("Ran ")) {
        unittest_failures(lines, &mut out);
    } else { return None; }
    Some(out.join("\n") + "\n")
}
