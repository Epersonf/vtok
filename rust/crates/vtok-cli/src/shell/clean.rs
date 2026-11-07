//! Line hygiene: ANSI/progress stripping, a signature for near-duplicate detection, a hard cap per line.
pub(super) const LINE_CAP: usize = 300;

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
pub(super) fn sig(line: &str) -> String {
    let mut s = String::with_capacity(line.len());
    let mut prev_hash = false;
    for c in line.trim().chars() {
        if c.is_ascii_digit() { if !prev_hash { s.push('#'); prev_hash = true; } } else { s.push(c); prev_hash = false; }
    }
    s
}

pub(super) fn cap(s: &str) -> String {
    if s.chars().count() <= LINE_CAP { s.to_string() } else { format!("{}…", s.chars().take(LINE_CAP).collect::<String>()) }
}
