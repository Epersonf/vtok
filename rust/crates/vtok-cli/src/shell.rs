//! `vtok run -- CMD` and `vtok output ID`: run a command, keep its full output on disk, show a compact view.
//!
//! No model involved. The compact view keeps the head, the tail, every error/warning/summary line with a little
//! context, and collapses runs of near-identical lines. Nothing is lost: the whole (ANSI-stripped) output is stored
//! under an id and `vtok output ID` pulls back any part of it. If the compact view would not be smaller, or the
//! output is short, the raw output is printed unchanged.
use std::collections::BTreeSet;
use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use vtok_api::ntok;

const RAW_BELOW_TOKENS: usize = 250;
const DEFAULT_BUDGET: usize = 700;
const MAX_CAPTURE: usize = 64 * 1024 * 1024;
const KEEP_FILES: usize = 100;
const KEEP_SECS: u64 = 7 * 24 * 3600;
const GREP_LINES: usize = 40;
/// `File "/a/b/m.py", line 9, in f` -> `m.py:9`
fn short_frame(l: &str) -> String {
    let Some(rest) = l.strip_prefix("File \"") else { return l.to_string() };
    let Some((path, tail)) = rest.split_once('"') else { return l.to_string() };
    let line = tail.trim_start_matches(", line ").split(',').next().unwrap_or("");
    format!("{}:{}", path.rsplit('/').next().unwrap_or(path), line)
}

/// Build-tool chatter that never decides anything (kept in the stored full output).
fn is_noise(l: &str) -> bool {
    let t = l.trim_start();
    ["Compiling ", "Finished ", "Running ", "Downloaded ", "Downloading ", "Doc-tests ", "Blocking waiting", "Updating ", "Locking "].iter().any(|p| t.starts_with(p))
        || t.starts_with("running ") && t.ends_with(" tests")
        || t.starts_with("note: run with `RUST_BACKTRACE")
        || t.starts_with("test session starts") || t.starts_with("Traceback (most recent call last)")
        || t.starts_with("platform ") || t.starts_with("rootdir:") || t.starts_with("collecting ")
        || !t.is_empty() && t.chars().all(|c| matches!(c, '^' | '~' | ' '))
}

fn without_noise(lines: &[String]) -> Vec<String> {
    let kept: Vec<String> = lines.iter().filter(|l| !is_noise(l)).cloned().collect();
    if kept.is_empty() { lines.to_vec() } else { kept }
}

/// `path:line:text` (grep -n / rg) -> (path, text)
fn grep_hit(l: &str) -> Option<(&str, &str)> {
    let (path, rest) = l.split_once(':')?;
    let (num, text) = rest.split_once(':')?;
    (!path.is_empty() && (path.contains('/') || path.contains('.')) && !path.contains(' ') && !num.is_empty() && num.bytes().all(|b| b.is_ascii_digit())).then_some((path, text))
}

fn is_path_line(l: &str) -> bool { !l.trim().is_empty() && !l.contains(' ') && l.contains('/') && !l.contains(':') }

/// Listings (find, grep -rn, ls -R) are structure, not prose: group by directory, as deep as the budget allows.
fn summarize_listing(lines: &[String], budget: usize) -> Option<String> {
    let rows: Vec<&String> = lines.iter().filter(|l| !l.trim().is_empty()).collect();
    if rows.len() < 20 { return None; }
    let greps = rows.iter().filter(|l| grep_hit(l).is_some()).count();
    let paths = rows.iter().filter(|l| is_path_line(l)).count();
    let hits = greps * 10 >= rows.len() * 8;
    if !hits && paths * 10 < rows.len() * 8 { return None; }
    let items: Vec<(&str, &str)> = rows.iter().filter_map(|l| if hits { grep_hit(l) } else { is_path_line(l).then_some((l.as_str(), "")) }).collect();
    let comps = |p: &str| p.split('/').filter(|c| !c.is_empty() && *c != ".").count();
    let max_d = items.iter().map(|(p, _)| comps(p)).max().unwrap_or(1);
    let key_at = |p: &str, d: usize| -> String {
        let parts: Vec<&str> = p.split('/').filter(|c| !c.is_empty() && *c != ".").collect();
        let lead = if p.starts_with('/') { "/" } else { "" };
        let cut = if parts.len() <= d { parts.len() - 1 } else { d };
        let dir = parts[..cut].join("/");
        if dir.is_empty() { format!("{lead}.") } else { format!("{lead}{dir}/") }
    };
    let parts_of = |p: &str| -> Vec<String> { p.split('/').filter(|c| !c.is_empty() && *c != ".").map(String::from).collect() };
    let first = parts_of(items[0].0);
    let mut common = first.len().saturating_sub(1);
    for (p, _) in &items { let q = parts_of(p); common = common.min(first.iter().zip(&q).take_while(|(a, b)| a == b).count()).min(q.len().saturating_sub(1)); }
    let mut best = None;
    for d in (common + 1)..=max_d.max(common + 1) {
        let mut groups: Vec<(String, usize, &str)> = vec![];
        let mut idx: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for (p, t) in &items {
            let k = key_at(p, d);
            match idx.get(&k) {
                Some(&i) => groups[i].1 += 1,
                None => { idx.insert(k.clone(), groups.len()); groups.push((k, 1, t)); }
            }
        }
        let mut out = String::new();
        for (k, n, t) in &groups {
            let what = if hits { "matches" } else { "files" };
            out.push_str(&format!("{k}  {n} {what}"));
            if hits { let t = t.trim(); out.push_str(&format!("   e.g. {}", cap(t))); }
            out.push('\n');
        }
        if ntok(&out) > budget {
            if best.is_none() {
                // even the coarsest grouping is too long: keep the biggest groups
                let mut ls: Vec<&str> = out.lines().collect();
                let total = ls.len();
                ls.sort_by_key(|l| std::cmp::Reverse(l.split_whitespace().nth(1).and_then(|n| n.parse::<usize>().ok()).unwrap_or(0)));
                let mut keep = String::new();
                let mut shown = 0;
                for l in ls { if ntok(&keep) + ntok(l) > budget { break; } keep.push_str(l); keep.push('\n'); shown += 1; }
                if shown < total { keep.push_str(&format!("... {} more groups\n", total - shown)); }
                best = Some(keep);
            }
            break;
        }
        best = Some(out);
    }
    let kind = if hits { "grep matches" } else { "paths" };
    best.map(|b| format!("{} {kind} in total, grouped by directory (full list: vtok output ID --grep TEXT)\n{b}", items.len()))
}

/// When something failed, passing-test lines only add noise: drop them and say how many.
fn without_passes(lines: &[String]) -> Vec<String> {
    let kept: Vec<String> = lines.iter().filter(|l| !is_pass_line(l)).cloned().collect();
    let dropped = lines.len() - kept.len();
    if dropped < 3 { return lines.to_vec(); }
    let mut out = kept;
    let at = out.iter().position(|l| score(l) >= 2).unwrap_or(0).min(out.len());
    out.insert(at, format!("({dropped} passing-test lines omitted)"));
    out
}

/// A passing test line whose name merely contains a word like "error" (`test x_keeps_errors ... ok`).
fn is_pass_line(l: &str) -> bool {
    let t = l.trim_end();
    t.ends_with(" ... ok") || t.ends_with(" PASSED") || t.contains(" PASSED ")
}

/// `cargo test` prints one `test result:` line per suite; add them up into one line.
fn sum_test_results(lines: &[String]) -> Option<String> {
    let (mut p, mut f, mut ig, mut suites) = (0u64, 0u64, 0u64, 0u64);
    for l in lines.iter().filter(|l| l.starts_with("test result:")) {
        suites += 1;
        for part in l.split(';') {
            let words: Vec<&str> = part.split_whitespace().collect();
            let Some(k) = words.iter().position(|w| matches!(*w, "passed" | "failed" | "ignored")) else { continue };
            let Some(n) = k.checked_sub(1).and_then(|k| words[k].parse::<u64>().ok()) else { continue };
            match words[k] { "passed" => p += n, "failed" => f += n, _ => ig += n }
        }
    }
    (suites > 0).then(|| {
        let mut parts = vec![format!("{p} passed")];
        if f > 0 { parts.push(format!("{f} failed")); }
        if ig > 0 { parts.push(format!("{ig} ignored")); }
        format!("test result: {}. {} ({suites} suites)", if f > 0 { "FAILED" } else { "ok" }, parts.join(", "))
    })
}

/// Successful run: the aggregated counts, other summary lines and the last lines, nothing else.
fn summarize_clean(lines: &[String], budget: usize) -> String {
    let agg = sum_test_results(lines);
    let lines: Vec<String> = lines.iter().filter(|l| !is_pass_line(l)).cloned().collect();
    let lines = &lines[..];
    let rest: Vec<String> = if agg.is_some() { lines.iter().filter(|l| !l.starts_with("test result:") && !l.starts_with("test ") && !l.starts_with("running ")).cloned().collect() } else { lines.to_vec() };
    let gs = collapse(&rest);
    let n = gs.len();
    let has_sum = gs.iter().any(|g| score(&g.text) == 2);
    let mut sel: BTreeSet<usize> = if has_sum { BTreeSet::new() } else { (n.saturating_sub(2)..n).collect() };
    for i in (0..n).rev() {
        if score(&gs[i].text) != 2 { continue; }
        let mut trial = sel.clone();
        trial.insert(i);
        if ntok(&render(&gs, &trial)) > budget { break; }
        sel = trial;
    }
    let body: String = sel.iter().map(|&i| { let g = &gs[i]; let c = if g.count > 1 { format!("  [×{}]", g.count) } else { String::new() }; format!("{}{c}\n", cap(g.text.trim_matches(|ch: char| ch == '=' || ch == '-' || ch == ' '))) }).collect();
    match agg { Some(a) => format!("{a}\n{body}"), None => body }
}

const CLEAN_BUDGET: usize = 150;
const LINE_CAP: usize = 300;

pub fn state_dir() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("VTOK_STATE") { return Some(PathBuf::from(p)); }
    if let Some(x) = std::env::var_os("XDG_STATE_HOME") { return Some(PathBuf::from(x).join("vtok")); }
    std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/state/vtok"))
}

fn out_dir() -> Option<PathBuf> { state_dir().map(|d| d.join("out")) }

pub fn valid_id(id: &str) -> bool { (6..=32).contains(&id.len()) && id.bytes().all(|b| b.is_ascii_hexdigit()) }

// ---------------------------------------------------------------- cleaning

/// Strip ANSI escapes and keep only the last `\r` segment of each line (progress bars).
pub fn clean(raw: &str) -> Vec<String> {
    let mut lines = vec![];
    for l in raw.split('\n') {
        let seg = l.rsplit('\r').find(|s| !s.is_empty()).unwrap_or("");
        let mut out = String::with_capacity(seg.len());
        let mut it = seg.chars().peekable();
        while let Some(c) = it.next() {
            if c == '\u{1b}' {
                match it.peek() {
                    Some('[') => { it.next(); while let Some(&n) = it.peek() { it.next(); if ('@'..='~').contains(&n) { break; } } }
                    Some(']') => { it.next(); while let Some(n) = it.next() { if n == '\u{7}' { break; } if n == '\u{1b}' { it.next(); break; } } }
                    _ => { it.next(); }
                }
            } else { out.push(c); }
        }
        lines.push(out.trim_end().to_string());
    }
    if lines.last().map_or(false, |l| l.is_empty()) { lines.pop(); }
    lines
}

/// Line signature for near-duplicate detection: digit runs and hex addresses collapse to `#`.
fn sig(line: &str) -> String {
    let mut s = String::with_capacity(line.len());
    let mut prev_hash = false;
    for c in line.trim().chars() {
        if c.is_ascii_digit() { if !prev_hash { s.push('#'); prev_hash = true; } } else { s.push(c); prev_hash = false; }
    }
    s
}

#[derive(Clone)]
struct Group { first: usize, last: usize, text: String, count: usize }

fn collapse(lines: &[String]) -> Vec<Group> {
    let mut gs: Vec<Group> = vec![];
    let mut last_sig = String::new();
    for (i, l) in lines.iter().enumerate() {
        let s = sig(l);
        if let Some(g) = gs.last_mut() {
            if !s.is_empty() && s == last_sig { g.last = i; g.count += 1; continue; }
        }
        last_sig = s;
        gs.push(Group { first: i, last: i, text: l.clone(), count: 1 });
    }
    dedupe_summaries(gs)
}

/// Non-adjacent summary lines that differ only in numbers (`test result: ok. 0 passed ...` once per crate) are
/// shown once with a total count; error lines are never merged.
fn dedupe_summaries(gs: Vec<Group>) -> Vec<Group> {
    let mut first: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut out: Vec<Group> = vec![];
    for g in gs {
        if score(&g.text) == 2 {
            let mut k = sig(&g.text);for w in g.text.split_whitespace(){let w=w.trim_end_matches(|c:char|!c.is_ascii_digit());if!w.is_empty()&&w!="0"&&w.bytes().all(|b|b.is_ascii_digit()){k.push('|');k.push_str(w);}}
            if let Some(&i) = first.get(&k) { out[i].count += g.count; continue; }
            first.insert(k, out.len());
        }
        out.push(g);
    }
    out
}

// ---------------------------------------------------------------- scoring

const ERR: &[&str] = &["error", "fail", "panic", "exception", "traceback", "fatal", "denied", "cannot ", "can't ", "not found", "assertion", "assert ", "✗", "segmentation", "abort", "timed out", "timeout", "unresolved"];
const SUM: &[&str] = &["test result", " passed", "passed;", "tests ran", "ran ", "summary", "total", "finished", "complete", "success", "test suites:", "failures:", "errors:", "real\t", "elapsed"];
const WARN: &[&str] = &["warning", "warn:", "deprecated"];

/// Drop "zero" counters so `0 failed` / `errors: 0` do not look like failures (`10 failed` is kept).
fn without_zero_counts(l: &str) -> String {
    let mut s = l.to_string();
    for w in ["failed", "failures", "failure", "errors", "error", "warnings", "warning", "errored"] {
        for pat in [format!("0 {w}"), format!("{w}: 0"), format!("{w}=0"), format!("{w} = 0")] {
            let mut from = 0;
            while let Some(i) = s[from..].find(&pat) {
                let at = from + i;
                let before_digit = s[..at].chars().last().map_or(false, |c| c.is_ascii_digit() || c == '.');
                let after_digit = s[at + pat.len()..].chars().next().map_or(false, |c| c.is_ascii_digit());
                if before_digit || after_digit { from = at + pat.len(); continue; }
                s.replace_range(at..at + pat.len(), "");
                from = at;
            }
        }
    }
    s
}

fn score(line: &str) -> u8 {
    let l = without_zero_counts(&line.to_lowercase());
    if ERR.iter().any(|k| l.contains(k)) { 3 }
    else if SUM.iter().any(|k| l.contains(k)) || l.trim_start().starts_with("tests:") { 2 }
    else if WARN.iter().any(|k| l.contains(k)) { 1 }
    else { 0 }
}

fn cap(s: &str) -> String {
    if s.chars().count() <= LINE_CAP { s.to_string() } else { format!("{}…", s.chars().take(LINE_CAP).collect::<String>()) }
}

fn render(gs: &[Group], sel: &BTreeSet<usize>) -> String {
    let mut out = String::new();
    let mut prev: Option<usize> = None;
    for &i in sel {
        match prev {
            Some(p) if p + 1 == i => {}
            Some(p) => out.push_str(&format!("… {} lines omitted …\nL{}:\n", gs[i].first - gs[p].last - 1, gs[i].first + 1)),
            None if gs[i].first > 0 => out.push_str(&format!("L{}:\n", gs[i].first + 1)),
            None => {}
        }
        let g = &gs[i];
        if g.text.is_empty() { prev = Some(i); continue; }
        out.push_str(&cap(&g.text));
        if g.count > 1 { out.push_str(&format!("  [×{} similar]", g.count)); }
        out.push('\n');
        prev = Some(i);
    }
    if let Some(p) = prev {
        let total = gs.last().map_or(0, |g| g.last);
        if gs[p].last < total { out.push_str(&format!("… {} lines omitted …\n", total - gs[p].last)); }
    }
    out
}

/// Compact view of `lines` that fits `budget` tokens, or None when it cannot beat the raw text.
fn summarize(lines: &[String], budget: usize) -> String {
    let gs = collapse(lines);
    let n = gs.len();
    let scores: Vec<u8> = gs.iter().map(|g| score(&g.text)).collect();
    for (head, tail, ctx) in [(0usize, 1usize, 1usize), (0, 1, 0)] {
        let mut sel: BTreeSet<usize> = BTreeSet::new();
        for i in 0..head.min(n) { sel.insert(i); }
        for i in n.saturating_sub(tail)..n { sel.insert(i); }
        let mut by_score: Vec<usize> = (0..n).filter(|&i| scores[i] > 0).collect();
        by_score.sort_by_key(|&i| (std::cmp::Reverse(scores[i]), i));
        let mut best = render(&gs, &sel);
        for i in by_score {
            let mut trial = sel.clone();
            let c = if scores[i] >= 3 { ctx } else { 0 };
            for j in i.saturating_sub(c)..(i + c + 1).min(n) { if j == i || !(gs[j].count > 1 && scores[j] == 0) { trial.insert(j); } }
            let r = render(&gs, &trial);
            if ntok(&r) > budget { if scores[i] >= 3 { continue; } else { break; } }
            sel = trial;
            best = r;
        }
        if ntok(&best) <= budget { return best; }
    }
    render(&gs, &(n.saturating_sub(1)..n).collect())
}

// ---------------------------------------------------------------- storage

fn fnv(s: &[u8]) -> u64 { let mut h = 0xcbf29ce484222325u64; for b in s { h ^= *b as u64; h = h.wrapping_mul(0x100000001b3); } h }

fn store(text: &str) -> Option<String> {
    let dir = out_dir()?;
    std::fs::create_dir_all(&dir).ok()?;
    #[cfg(unix)]
    { use std::os::unix::fs::PermissionsExt; let _ = std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)); }
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).ok()?.as_nanos();
    let id = format!("{:08x}", (fnv(text.as_bytes()) ^ (nanos as u64).rotate_left(17)) & 0xffff_ffff);
    let path = dir.join(format!("{id}.txt"));
    std::fs::write(&path, text).ok()?;
    #[cfg(unix)]
    { use std::os::unix::fs::PermissionsExt; let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)); }
    gc(&dir);
    Some(id)
}

fn gc(dir: &std::path::Path) {
    let mut files: Vec<(std::time::SystemTime, PathBuf)> = std::fs::read_dir(dir).into_iter().flatten().flatten()
        .filter(|e| e.path().extension().map_or(false, |x| x == "txt"))
        .filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path()))).collect();
    files.sort_by(|a, b| b.0.cmp(&a.0));
    let now = std::time::SystemTime::now();
    for (i, (t, p)) in files.iter().enumerate() {
        let old = now.duration_since(*t).map_or(false, |d| d.as_secs() > KEEP_SECS);
        if i >= KEEP_FILES || old { let _ = std::fs::remove_file(p); }
    }
}

pub fn load(id: &str) -> Result<Vec<String>, String> {
    if !valid_id(id) { return Err(format!("invalid output id {id:?}")); }
    let dir = out_dir().ok_or("no state directory (set HOME or VTOK_STATE)")?;
    let text = std::fs::read_to_string(dir.join(format!("{id}.txt"))).map_err(|_| format!("no stored output {id:?} (expired or never stored)"))?;
    Ok(text.split('\n').map(String::from).collect::<Vec<_>>().into_iter().rev().skip_while(|l| l.is_empty()).collect::<Vec<_>>().into_iter().rev().collect())
}

// ---------------------------------------------------------------- run

/// What `vtok run` prints for a finished command (and stores). Pure apart from the storage side effect.
/// pytest and cargo test failures reduced to what decides the fix: which test, where, and the assertion.
fn failure_view(lines: &[String]) -> Option<String> {
    let mut out: Vec<String> = vec![];
    if let Some(start) = lines.iter().position(|l| l.contains(" FAILURES ") && l.starts_with('=')) {
        // one block per failing test: (name, lines)
        let mut blocks: Vec<(String, Vec<String>)> = vec![];
        for l in &lines[start + 1..] {
            let t = l.trim();
            if t.contains("short test summary info") { break; }
            if t.len() > 6 && t.starts_with("___") && t.ends_with("___") { let n = t.trim_matches('_').trim().to_string(); blocks.push((n.clone(), vec![format!("FAIL {n}")])); }
            else if let Some(b) = blocks.last_mut() {
                if t.starts_with("E ") || t == "E" { b.1.push(format!("  {}", cap(t))); }
                else if let Some((f, rest)) = t.split_once(':') {
                    if f.ends_with(".py") && rest.split(':').next().map_or(false, |n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit())) { b.1.push(format!("  at {}", cap(t))); }
                }
            }
        }
        // parametrized failures share a base name: keep the first, plus any whose assertion has a different shape (digits ignored)
        let shape = |b: &[String]| b.iter().skip(1).filter(|l| l.trim_start().starts_with("E ")).map(|l| { let mut o = String::new(); for c in l.chars() { if c.is_ascii_digit() { if !o.ends_with('#') { o.push('#'); } } else { o.push(c); } } o }).collect::<Vec<_>>().join("|");
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
    } else if lines.iter().any(|l| l.starts_with("---- ") && l.ends_with(" stdout ----")) {
        let mut inb = false;
        for l in lines {
            if l.starts_with("---- ") && l.ends_with(" stdout ----") { inb = true; out.push(l.trim_matches('-').trim().to_string()); continue; }
            if l == "failures:" || l.starts_with("test result:") || l.starts_with("error: test failed") { inb = false; continue; }
            if inb && !l.trim().is_empty() && !l.starts_with("note: run with") { out.push(format!("  {}", cap(l.trim()))); }
        }
        out.push(sum_test_results(lines)?);
    } else if lines.iter().any(|l| l.starts_with("Traceback") ) && lines.iter().any(|l| l.starts_with("FAIL: ") || l.starts_with("ERROR: ")) && lines.iter().any(|l| l.starts_with("Ran ")) {
        // unittest: one block per failure -> header, innermost frame, exception line
        let mut frame: Option<String> = None;
        let mut head: Option<String> = None;
        let flush = |out: &mut Vec<String>, head: &mut Option<String>, frame: &mut Option<String>, exc: Option<&str>| {
            if let Some(h) = head.take() { out.push(h); if let Some(f) = frame.take() { out.push(format!("  {f}")); } if let Some(e) = exc { out.push(format!("  {}", cap(e))); } }
        };
        for l in lines {
            if l.starts_with("FAIL: ") || l.starts_with("ERROR: ") { head = Some(l.clone()); frame = None; }
            else if head.is_some() && l.trim_start().starts_with("File \"") { frame = Some(short_frame(l.trim())); }
            else if head.is_some() && !l.starts_with(' ') && !l.starts_with('-') && !l.starts_with('=') && !l.is_empty() && !l.starts_with("Traceback") && l.contains(':') { flush(&mut out, &mut head, &mut frame, Some(l)); }
        }
        flush(&mut out, &mut head, &mut frame, None);
        for l in lines.iter().filter(|l| l.starts_with("Ran ") || l.starts_with("FAILED (") || l == &"OK") { out.push(l.clone()); }
    } else { return None; }
    Some(out.join("\n") + "\n")
}

/// Column-aligned tables: the padding is pure cost. Collapse inner runs of 3+ spaces and drop `---- ----` rulers.
fn squeeze_table(lines: &[String]) -> Option<Vec<String>> {
    let rows = lines.iter().filter(|l| !l.trim().is_empty()).count();
    let padded = |l: &str| l.trim_start().contains("   ");
    if rows < 5 || lines.iter().filter(|l| padded(l)).count() * 10 < rows * 6 { return None; }
    let sq = |l: &str| -> String {
        let t = l.trim_start();
        let lead = &l[..l.len() - t.len()];
        let mut out = String::from(lead);
        let mut run = 0;
        for c in t.chars() {
            if c == ' ' { run += 1; continue; }
            if run > 0 { if run >= 3 { out.push(' '); } else { out.push_str(&" ".repeat(run)); } run = 0; }
            out.push(c);
        }
        out
    };
    Some(lines.iter().filter(|l| !(l.len() > 3 && l.chars().all(|c| matches!(c, '-' | '=' | ' ')))).map(|l| sq(l)).collect())
}

pub fn compact(raw: &str, code: i32, budget: usize) -> String {
    let orig_lines = clean(raw);
    let orig = orig_lines.join("\n");
    let lines = squeeze_table(&orig_lines).unwrap_or_else(|| orig_lines.clone());
    let text = lines.join("\n");
    let raw_tok = ntok(&orig);
    // a clean run (exit 0, no error lines) needs only its summary, not a full head/tail
    let clean_run = code == 0 && !lines.iter().any(|l| score(l) >= 3 && !is_pass_line(l));
    let runner = lines.iter().any(|l| score(l) == 2);
    if raw_tok <= RAW_BELOW_TOKENS && !(clean_run && runner && raw_tok > 60) { return if text.is_empty() { text } else { text + "\n" }; }
    let git = crate::gitfmt::diff_view(&lines, budget).or_else(|| crate::gitfmt::log_view(&lines, budget));
    let test_report = lines.iter().any(|l| l.starts_with('=') && l.contains(" FAILURES "));
    let body = if let Some(g) = git { g }
        else if let Some(v) = failure_view(&lines).filter(|v| test_report && ntok(v) <= budget) { v }
        else if let Some(l) = summarize_listing(&lines, budget.min(150)) { l }
        else if clean_run && runner { summarize_clean(&without_noise(&lines), budget.min(CLEAN_BUDGET)) }
        else if let Some(v) = failure_view(&lines).filter(|v| ntok(v) <= budget) { v }
        else { summarize(&without_noise(&without_passes(&lines)), budget) };
    let Some(id) = store(&(orig.clone() + "\n")) else { return text + "\n" };
    let quiet = clean_run && runner;
    let head = if quiet { format!("# ok id={id}\n") } else { format!("# exit={code} id={id}\n") };
    let foot = if quiet { String::new() } else { format!("# more: vtok output {id}\n") };
    let shown = format!("{head}{body}{foot}");
    if ntok(&shown) >= raw_tok {
        crate::usage::record("run", raw_tok, raw_tok, "(not smaller: raw)");
        text + "\n"
    } else {
        crate::usage::record("run", raw_tok, ntok(&shown), &id);
        shown
    }
}

pub struct Ran { pub out: Vec<u8>, pub code: i32, pub timed_out: bool }

/// Run `cmd` (one string = a shell snippet, several = argv) with stderr merged into stdout.
pub fn execute(cmd: &[String], cwd: Option<&str>, timeout: Option<std::time::Duration>, inherit_stdin: bool) -> Result<Ran, String> {
    let mut c = Command::new("sh");
    if cmd.len() == 1 { c.arg("-c").arg(format!("{{ {}\n}} 2>&1", cmd[0])); }
    else { c.arg("-c").arg("exec \"$@\" 2>&1").arg("sh").args(cmd); }
    if let Some(d) = cwd { c.current_dir(d); }
    c.stdin(if inherit_stdin { Stdio::inherit() } else { Stdio::null() }).stdout(Stdio::piped()).stderr(Stdio::inherit());
    #[cfg(unix)]
    { use std::os::unix::process::CommandExt; c.process_group(0); }
    let mut child = c.spawn().map_err(|e| format!("cannot start: {e}"))?;
    let mut stdout = child.stdout.take().ok_or("no stdout")?;
    let reader = std::thread::spawn(move || {
        let mut buf = vec![];
        let _ = (&mut stdout).take(MAX_CAPTURE as u64).read_to_end(&mut buf);
        let _ = std::io::copy(&mut stdout, &mut std::io::sink());
        buf
    });
    let mut timed_out = false;
    let status = match timeout {
        None => child.wait().ok(),
        Some(t) => {
            let start = std::time::Instant::now();
            loop {
                match child.try_wait() {
                    Ok(Some(st)) => break Some(st),
                    Ok(None) if start.elapsed() >= t => {
                        timed_out = true;
                        let _ = Command::new("kill").args(["-KILL", "--", &format!("-{}", child.id())]).status();
                        let _ = child.kill();
                        break child.wait().ok();
                    }
                    Ok(None) => std::thread::sleep(std::time::Duration::from_millis(20)),
                    Err(_) => break None,
                }
            }
        }
    };
    let out = reader.join().unwrap_or_default();
    let code = status.map_or(1, |st| st.code().unwrap_or_else(|| {
        #[cfg(unix)] { use std::os::unix::process::ExitStatusExt; 128 + st.signal().unwrap_or(0) }
        #[cfg(not(unix))] { 1 }
    }));
    Ok(Ran { out, code, timed_out })
}

pub fn run(args: &[String]) -> i32 {
    let mut budget = DEFAULT_BUDGET;
    let mut i = 0;
    while i < args.len() && args[i] != "--" {
        match args[i].as_str() {
            "--max-tokens" => { i += 1; budget = args.get(i).and_then(|s| s.parse().ok()).unwrap_or_else(|| { eprintln!("--max-tokens needs a number"); std::process::exit(2) }); }
            other => { eprintln!("unknown option {other:?}\nusage: vtok run [--max-tokens N] -- COMMAND [ARGS...]"); return 2; }
        }
        i += 1;
    }
    let cmd: Vec<String> = args.iter().skip(i + 1).cloned().collect();
    if cmd.is_empty() { eprintln!("usage: vtok run [--max-tokens N] -- COMMAND [ARGS...]"); return 2; }
    match execute(&cmd, None, None, true) {
        Ok(r) => { print!("{}", compact(&String::from_utf8_lossy(&r.out), r.code, budget)); r.code }
        Err(e) => { eprintln!("vtok run: {e}"); 127 }
    }
}

/// `vtok_run` for the MCP server: same compact view, always led by the exit status.
pub fn run_for_mcp(command: &str, cwd: Option<&str>, timeout_s: u64, budget: usize) -> Result<String, String> {
    let r = execute(&[command.to_string()], cwd, Some(std::time::Duration::from_secs(timeout_s.clamp(1, 900))), false)?;
    let body = compact(&String::from_utf8_lossy(&r.out), r.code, budget);
    let status = if r.timed_out { format!("[timed out after {timeout_s}s, killed; exit {}]", r.code) } else { format!("[exit {}]", r.code) };
    Ok(format!("{status}\n{body}"))
}

// ---------------------------------------------------------------- output

pub struct Query { pub grep: Vec<String>, pub range: Option<(usize, usize)>, pub tail: Option<usize>, pub head: Option<usize>, pub context: usize, pub max_tokens: usize }

pub fn fetch(id: &str, q: &Query) -> Result<String, String> {
    let lines = load(id)?;
    let n = lines.len();
    let mut sel: BTreeSet<usize> = BTreeSet::new();
    let mut grep_total = 0;
    if !q.grep.is_empty() {
        let needles: Vec<String> = q.grep.iter().map(|s| s.to_lowercase()).collect();
        for (i, l) in lines.iter().enumerate() {
            let low = l.to_lowercase();
            if needles.iter().any(|k| low.contains(k.as_str())) { for j in i.saturating_sub(q.context)..(i + q.context + 1).min(n) { sel.insert(j); } }
        }
        grep_total = sel.len();
        if grep_total > GREP_LINES { sel = sel.into_iter().take(GREP_LINES).collect(); }
    }
    if let Some((a, b)) = q.range { for j in a.max(1) - 1..b.min(n) { sel.insert(j); } }
    if let Some(t) = q.tail { for j in n.saturating_sub(t)..n { sel.insert(j); } }
    if let Some(h) = q.head { for j in 0..h.min(n) { sel.insert(j); } }
    if sel.is_empty() && q.grep.is_empty() && q.range.is_none() && q.tail.is_none() && q.head.is_none() { for j in n.saturating_sub(40)..n { sel.insert(j); } }
    let mut out = format!("# output {id}: {n} lines\n");
    let mut prev: Option<usize> = None;
    let mut used = ntok(&out);
    for j in sel {
        if let Some(p) = prev { if p + 1 != j { out.push_str("--\n"); } }
        let line = format!("{}: {}\n", j + 1, cap(&lines[j]));
        used += ntok(&line);
        if used > q.max_tokens { out.push_str(&format!("# truncated at ~{} tokens; narrow with --grep / --lines\n", q.max_tokens)); break; }
        out.push_str(&line);
        prev = Some(j);
    }
    if grep_total > GREP_LINES { out.push_str(&format!("# grep: showing the first {GREP_LINES} of {grep_total} lines; narrow the pattern or use --lines\n")); }
    if prev.is_none() { out.push_str("# no matching lines\n"); }
    Ok(out)
}

pub fn parse_range(s: &str) -> Option<(usize, usize)> {
    let (a, b) = s.split_once('-').unwrap_or((s, s));
    Some((a.trim().parse().ok()?, b.trim().parse().ok()?))
}

pub fn output_cmd(args: &[String]) -> i32 {
    let mut id = None;
    let mut q = Query { grep: vec![], range: None, tail: None, head: None, context: 2, max_tokens: 2000 };
    let mut i = 0;
    while i < args.len() {
        let next = |i: &mut usize| -> Option<String> { *i += 1; args.get(*i).cloned() };
        match args[i].as_str() {
            "--grep" => { if let Some(v) = next(&mut i) { q.grep.extend(v.split('|').map(String::from)); } }
            "--lines" => { q.range = next(&mut i).and_then(|v| parse_range(&v)); }
            "--tail" => { q.tail = next(&mut i).and_then(|v| v.parse().ok()); }
            "--head" => { q.head = next(&mut i).and_then(|v| v.parse().ok()); }
            "--context" => { q.context = next(&mut i).and_then(|v| v.parse().ok()).unwrap_or(2); }
            "--max-tokens" => { q.max_tokens = next(&mut i).and_then(|v| v.parse().ok()).unwrap_or(2000); }
            a if !a.starts_with("--") && id.is_none() => id = Some(a.to_string()),
            a => { eprintln!("unknown argument {a:?}"); return 2; }
        }
        i += 1;
    }
    let Some(id) = id else { eprintln!("usage: vtok output ID [--grep A|B] [--lines A-B] [--tail N] [--head N] [--context N] [--max-tokens N]"); return 2 };
    match fetch(&id, &q) { Ok(s) => { print!("{s}"); 0 } Err(e) => { eprintln!("error: {e}"); 1 } }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_ansi_and_progress() {
        let l = clean("\u{1b}[31mred\u{1b}[0m text\nprogress 10%\rprogress 50%\rprogress 100%\n");
        assert_eq!(l, vec!["red text".to_string(), "progress 100%".to_string()]);
    }

    #[test]
    fn collapses_similar_lines_and_keeps_errors() {
        let mut raw = String::new();
        for i in 0..400 { raw.push_str(&format!("compiling crate-{i} v0.{i}.1\n")); }
        raw.push_str("error[E0308]: mismatched types\n  --> src/lib.rs:10:5\n");
        for i in 0..400 { raw.push_str(&format!("step {i} done ok ok ok\n")); }
        raw.push_str("test result: FAILED. 3 passed; 1 failed\n");
        let lines = clean(&raw);
        let s = summarize(&lines, 400);
        assert!(s.contains("mismatched types"), "{s}");
        assert!(s.contains("test result: FAILED"), "{s}");
        assert!(!s.contains("compiling crate-17"), "repeated lines must not be listed one by one: {s}");
        assert!(ntok(&s) <= 400 + 40, "{}", ntok(&s));
    }

    #[test]
    fn zero_counters_are_not_failures() {
        assert_eq!(score("test result: ok. 4 passed; 0 failed; 0 ignored"), 2);
        assert_eq!(score("test result: FAILED. 3 passed; 10 failed"), 3);
        assert_eq!(score("Tests: 5 passed, errors: 0"), 2);
        assert_eq!(score("error[E0308]: mismatched types"), 3);
    }

    #[test]
    fn short_output_is_untouched() {
        assert_eq!(compact("hello\nworld\n", 0, 700), "hello\nworld\n");
    }

    #[test]
    fn ids_are_validated() {
        assert!(valid_id("deadbeef"));
        assert!(!valid_id("../etc/passwd"));
        assert!(!valid_id("abc"));
    }

    #[test]
    fn nonzero_summaries_survive_dedupe() {
        let mut raw = String::new();
        for n in [0, 4, 0, 7, 0] {
            raw.push_str(&format!("   Running unit {n}\ntest result: ok. {n} passed; 0 failed; finished in 0.0{n}s\n\n"));
        }
        let out = summarize(&clean(&raw), 700);
        assert!(out.contains("4 passed") && out.contains("7 passed"), "{out}");
    }

    #[test]
    fn listings_are_grouped_by_directory() {
        let raw: Vec<String> = (0..60).map(|i| format!("./src/mod{}/file{i}.rs", i % 3)).collect();
        let out = summarize_listing(&raw, 300).unwrap();
        assert!(out.contains("60 paths") && out.contains("./src/mod0/") || out.contains("src/mod0/"), "{out}");
        let hits: Vec<String> = (0..40).map(|i| format!("a/b{}.py:{}:def f{i}():", i % 2, i + 1)).collect();
        assert!(summarize_listing(&hits, 300).unwrap().contains("40 grep matches"));
        let log: Vec<String> = (0..40).map(|i| format!("2026-11-06T10:00:{i:02}Z INFO ok")).collect();
        assert!(summarize_listing(&log, 300).is_none());
    }

    #[test]
    fn test_results_are_summed_and_failures_drop_passing_noise() {
        let l = |t: &[&str]| t.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        let sums = sum_test_results(&l(&["test result: ok. 4 passed; 0 failed; 1 ignored; 0 measured", "test result: ok. 3 passed; 0 failed; 0 ignored"])).unwrap();
        assert!(sums.contains("7 passed") && sums.contains("1 ignored") && sums.contains("2 suites"), "{sums}");
        let mut raw: Vec<String> = (0..10).map(|i| format!("test t{i} ... ok")).collect();
        raw.push("test bad ... FAILED".into());
        let w = without_passes(&raw);
        assert!(w.iter().any(|x| x.contains("10 passing-test lines omitted")) && w.iter().any(|x| x.contains("bad")));
    }

    #[test]
    fn failure_views_keep_name_place_and_assertion() {
        let l = |t: &str| t.lines().map(String::from).collect::<Vec<_>>();
        let py = l("=== FAILURES ===\n____ test_a ____\n    def test_a():\n>       assert 1 == 2\nE       assert 1 == 2\ntest_x.py:3: AssertionError\n=== short test summary info ===\nFAILED test_x.py::test_a - assert 1 == 2\n=== 1 failed, 5 passed in 0.1s ===");
        let v = failure_view(&py).unwrap();
        assert!(v.contains("FAIL test_a") && v.contains("E       assert 1 == 2") && v.contains("at test_x.py:3") && v.contains("1 failed, 5 passed"), "{v}");
        assert!(!v.contains("def test_a"));
        let mut many = String::from("=== FAILURES ===\n");
        for i in 0..20 { many += &format!("____ test_p[{i}] ____\n>   assert f({i}) == 1\nE   assert {} == 1\nt.py:5: AssertionError\n", i + 7); }
        many += "=== short test summary info ===\n=== 20 failed, 5 passed in 0.1s ===";
        let v = failure_view(&l(&many)).unwrap();
        assert!(v.matches("FAIL test_p").count() == 1 && v.contains("+19 more failures of test_p") && v.contains("20 failed"), "{v}");
        let ut = l("ERROR: test_k (m.T.test_k)\n----\nTraceback (most recent call last):\n  File \"/t/m.py\", line 9, in test_k\n    {}['k']\nKeyError: 'k'\n\nRan 3 tests in 0.1s\n\nFAILED (errors=1)");
        let v = failure_view(&ut).unwrap();
        assert!(v.contains("ERROR: test_k") && v.contains("m.py:9") && v.contains("KeyError") && v.contains("Ran 3 tests"), "{v}");
    }
}
