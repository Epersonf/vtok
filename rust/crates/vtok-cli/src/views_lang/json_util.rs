//! Helpers for views over JSON tool output: tolerant document parse, ANSI strip, message trimming.
use serde_json::Value;

/// First JSON document in the output; text before the first `{`/`[` line (banners, log lines) is skipped.
pub fn doc(lines: &[String]) -> Option<Value> {
    let start = lines.iter().position(|l| matches!(l.trim_start().chars().next(), Some('{') | Some('[')))?;
    let text = lines[start..].join("\n");
    serde_json::Deserializer::from_str(&text).into_iter::<Value>().next()?.ok()
}

/// Remove ANSI colour escapes.
pub fn ansi(s: &str) -> String {
    super::util::re(r"\x1b\[[0-9;]*[A-Za-z]").replace_all(s, "").into_owned()
}

/// First `n` non-empty lines, each cut at `w` chars.
pub fn first_lines(s: &str, n: usize, w: usize) -> String {
    s.lines().map(str::trim_end).filter(|l| !l.trim().is_empty()).take(n).map(|l| super::util::short(l, w)).collect::<Vec<_>>().join("\n")
}
