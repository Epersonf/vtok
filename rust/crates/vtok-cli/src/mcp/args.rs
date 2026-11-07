//! Shared by the tools: file revision hash, reading a supported source file, argument access.
use serde_json::Value;
use vtok_api::*;

pub(super) fn fnv(s: &str) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.as_bytes() { h ^= *b as u64; h = h.wrapping_mul(0x100000001b3); }
    format!("{h:016x}")
}

pub(super) fn read_source(path: &str) -> std::result::Result<(String, &'static dyn Language, bool), String> {
    let lang = language_for_path(path).map_err(|e| format!("{e} (supported: {}; use the normal read tool for this file)",languages().iter().flat_map(|l|l.extensions().iter().copied()).collect::<Vec<_>>().join(" ")))?;
    let raw = std::fs::read_to_string(path).map_err(|e| format!("cannot read {path}: {e}"))?;
    let crlf = raw.contains("\r\n");
    Ok((normalize_newlines(&raw), lang, crlf))
}

pub(super) fn arg_str<'a>(a: &'a Value, k: &str) -> std::result::Result<&'a str, String> {
    a.get(k).and_then(|v| v.as_str()).ok_or_else(|| format!("missing string argument {k:?}"))
}

pub(super) fn max_lines(a: &Value) -> usize {
    a.get("max_lines").and_then(|v| v.as_u64()).map(|n| n.max(1) as usize).unwrap_or_else(|| {
        crate::config::max_lines(a.get("path").and_then(|v| v.as_str()).map(std::path::Path::new)).0
    })
}
