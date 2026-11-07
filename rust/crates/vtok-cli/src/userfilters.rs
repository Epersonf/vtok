//! User-defined output filters for `vtok run`, for commands vtok does not know. Read from the user file only
//! (`$VTOK_FILTERS`, else next to the user config as `filters.json`): a repository can never hide output from its own
//! agent. Plain substring matching, no regex. Example `~/.config/vtok/filters.json`:
//!   [{"match": "make deploy", "drop_contains": ["Refreshing", "Reading"], "keep_contains": [], "head": 3, "tail": 10}]
//! `match` is a substring of the command line; lines containing any `drop_contains` string are removed; when
//! `keep_contains` is not empty only lines containing one of its strings stay; `head`/`tail` cap what is left.
use serde_json::Value;
use std::path::PathBuf;

pub fn path() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("VTOK_FILTERS") { return Some(PathBuf::from(p)); }
    crate::config::user_config_path().map(|p| p.with_file_name("filters.json"))
}

fn strings(v: &Value, key: &str) -> Vec<String> {
    v.get(key).and_then(|x| x.as_array()).map(|a| a.iter().filter_map(|s| s.as_str().map(String::from)).collect()).unwrap_or_default()
}

/// Apply the first user filter whose `match` occurs in `cmd`. None = no filter applies.
pub fn apply_with(filters: &Value, cmd: &str, lines: &[String]) -> Option<Vec<String>> {
    let f = filters.as_array()?.iter().find(|f| f.get("match").and_then(|m| m.as_str()).map_or(false, |m| !m.is_empty() && cmd.contains(m)))?;
    let (drop, keep) = (strings(f, "drop_contains"), strings(f, "keep_contains"));
    let mut out: Vec<String> = lines.iter().filter(|l| !drop.iter().any(|d| l.contains(d.as_str())) && (keep.is_empty() || keep.iter().any(|k| l.contains(k.as_str())))).cloned().collect();
    let (head, tail) = (f.get("head").and_then(|x| x.as_u64()).unwrap_or(0) as usize, f.get("tail").and_then(|x| x.as_u64()).unwrap_or(0) as usize);
    if head + tail > 0 && out.len() > head + tail + 1 {
        let hidden = out.len() - head - tail;
        let mut capped: Vec<String> = out[..head].to_vec();
        capped.push(format!("… {hidden} lines hidden"));
        capped.extend(out[out.len() - tail..].iter().cloned());
        out = capped;
    }
    Some(out)
}

pub fn apply(cmd: &str, lines: &[String]) -> Option<Vec<String>> {
    if cmd.is_empty() { return None; }
    let text = std::fs::read_to_string(path()?).ok()?;
    apply_with(&serde_json::from_str(&text).ok()?, cmd, lines)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters_drop_keep_and_cap() {
        let cfg: Value = serde_json::from_str(r#"[{"match":"make deploy","drop_contains":["Refreshing"],"head":1,"tail":1}]"#).unwrap();
        let lines: Vec<String> = ["Refreshing a", "one", "two", "three", "Refreshing b", "four"].iter().map(|s| s.to_string()).collect();
        let out = apply_with(&cfg, "make deploy prod", &lines).unwrap();
        assert_eq!(out, vec!["one", "… 2 lines hidden", "four"]);
        assert!(apply_with(&cfg, "make build", &lines).is_none());
        let keep: Value = serde_json::from_str(r#"[{"match":"x","keep_contains":["ERR"]}]"#).unwrap();
        assert_eq!(apply_with(&keep, "x", &["ok".into(), "ERR a".into()]).unwrap(), vec!["ERR a"]);
    }
}
