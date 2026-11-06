//! Persistent settings (`vtok config`). Today: `max-lines`, the size above which a file is split into chunks.
//!
//! Resolution order, first hit wins: `--max-lines` / `max_lines` argument, `VTOK_MAX_LINES`, the nearest
//! `.vtok.json` walking up from the file, the user file (`$VTOK_CONFIG`, else `$XDG_CONFIG_HOME/vtok/config.json`,
//! else `~/.config/vtok/config.json`), then 150.
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

pub static ALLOW_RUN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub const DEFAULT_MAX_LINES: usize = 150;
pub const MAX_LINES_LIMIT: usize = 100_000;
const KEY: &str = "max_lines";

pub fn user_config_path() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("VTOK_CONFIG") { return Some(PathBuf::from(p)); }
    if let Some(x) = std::env::var_os("XDG_CONFIG_HOME") { return Some(PathBuf::from(x).join("vtok/config.json")); }
    std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config/vtok/config.json"))
}

/// Nearest `.vtok.json` at or above `start` (a file or a directory); bounded to 64 ancestors.
pub fn local_config_path(start: &Path) -> Option<PathBuf> {
    let abs = if start.is_absolute() { start.to_path_buf() } else { std::env::current_dir().ok()?.join(start) };
    let mut dir = if abs.is_dir() { Some(abs.as_path()) } else { abs.parent() };
    for _ in 0..64 {
        let d = dir?;
        let f = d.join(".vtok.json");
        if f.is_file() { return Some(f); }
        dir = d.parent();
    }
    None
}

fn read_json(path: &Path) -> Value {
    std::fs::read_to_string(path).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_else(|| json!({}))
}

fn max_lines_in(path: &Path) -> Option<usize> {
    read_json(path).get(KEY).and_then(|v| v.as_u64()).map(|n| n as usize).filter(|n| (1..=MAX_LINES_LIMIT).contains(n))
}

/// Effective max-lines and where it came from. `near` is the file being worked on (for `.vtok.json` lookup).
pub fn max_lines(near: Option<&Path>) -> (usize, String) {
    if let Some(n) = std::env::var("VTOK_MAX_LINES").ok().and_then(|s| s.trim().parse::<usize>().ok()).filter(|n| (1..=MAX_LINES_LIMIT).contains(n)) {
        return (n, "VTOK_MAX_LINES".into());
    }
    let start = near.map(Path::to_path_buf).or_else(|| std::env::current_dir().ok());
    if let Some(p) = start.as_deref().and_then(local_config_path) {
        if let Some(n) = max_lines_in(&p) { return (n, p.display().to_string()); }
    }
    if let Some(p) = user_config_path() {
        if let Some(n) = max_lines_in(&p) { return (n, p.display().to_string()); }
    }
    (DEFAULT_MAX_LINES, "default".into())
}

fn target(local: bool) -> Result<PathBuf, String> {
    if local { Ok(std::env::current_dir().map_err(|e| e.to_string())?.join(".vtok.json")) }
    else { user_config_path().ok_or_else(|| "cannot locate the user config (set HOME or VTOK_CONFIG)".to_string()) }
}

fn write(path: &Path, v: &Value) -> Result<(), String> {
    if let Some(d) = path.parent() { std::fs::create_dir_all(d).map_err(|e| format!("{}: {e}", d.display()))?; }
    let text = serde_json::to_string_pretty(v).map_err(|e| e.to_string())? + "\n";
    std::fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))
}

/// Whether the MCP server may run commands (`vtok_run`). Off by default. Only the command line (`vtok mcp
/// --allow-run`), `VTOK_MCP_RUN=1`, or the USER config can turn it on - never a repo-local `.vtok.json`, so a cloned
/// repository cannot grant itself command execution.
pub fn mcp_run_enabled() -> bool {
    ALLOW_RUN.load(std::sync::atomic::Ordering::Relaxed)
        || std::env::var("VTOK_MCP_RUN").map_or(false, |v| v == "1")
        || user_config_path().map_or(false, |p| read_json(&p).get("mcp_run").and_then(|v| v.as_bool()) == Some(true))
}

pub fn set_mcp_run(on: bool) -> Result<PathBuf, String> {
    let path = target(false)?;
    let mut v = read_json(&path);
    if !v.is_object() { v = json!({}); }
    if on { v["mcp_run"] = json!(true); } else if let Some(o) = v.as_object_mut() { o.remove("mcp_run"); }
    write(&path, &v)?;
    Ok(path)
}

pub fn set_max_lines(n: usize, local: bool) -> Result<PathBuf, String> {
    if !(1..=MAX_LINES_LIMIT).contains(&n) { return Err(format!("max-lines must be between 1 and {MAX_LINES_LIMIT}")); }
    let path = target(local)?;
    let mut v = read_json(&path);
    if !v.is_object() { v = json!({}); }
    v[KEY] = json!(n);
    write(&path, &v)?;
    Ok(path)
}

pub fn unset_max_lines(local: bool) -> Result<PathBuf, String> {
    let path = target(local)?;
    let mut v = read_json(&path);
    if let Some(o) = v.as_object_mut() { o.remove(KEY); }
    if path.exists() { write(&path, &v)?; }
    Ok(path)
}

/// `vtok config [get|set|unset] ...` (args after the word `config`). Returns the exit code.
pub fn run(args: &[String]) -> i32 {
    let local = args.iter().any(|a| a == "--local");
    let words: Vec<&String> = args.iter().filter(|a| !a.starts_with("--")).collect();
    let w = |i: usize| words.get(i).map(|s| s.as_str());
    let key_ok = |k: Option<&str>| matches!(k, Some("max-lines") | Some("max_lines"));
    match w(0) {
        None | Some("show") => {
            let (n, src) = max_lines(None);
            println!("max-lines = {n}  ({src})");
            println!("mcp-run = {}  (lets the MCP server run commands via vtok_run; user-level only)", if mcp_run_enabled() { "on" } else { "off" });
            println!("user config: {}", user_config_path().map_or("-".into(), |p| p.display().to_string()));
            println!("repo config: {}", std::env::current_dir().ok().and_then(|d| local_config_path(&d)).map_or("-".into(), |p| p.display().to_string()));
            0
        }
        Some("set") if matches!(w(1), Some("mcp-run")) => {
            if local { eprintln!("error: mcp-run can only be set for the user, not with --local"); return 2; }
            match w(2) {
                Some("on") | Some("off") => match set_mcp_run(w(2) == Some("on")) {
                    Ok(p) => { println!("mcp-run = {}  ({})", w(2).unwrap_or("off"), p.display()); 0 }
                    Err(e) => { eprintln!("error: {e}"); 2 }
                },
                _ => { eprintln!("usage: vtok config set mcp-run on|off"); 2 }
            }
        }
        Some("get") if key_ok(w(1)) => { println!("{}", max_lines(None).0); 0 }
        Some("set") if key_ok(w(1)) => match w(2).and_then(|s| s.parse::<usize>().ok()) {
            Some(n) => match set_max_lines(n, local) {
                Ok(p) => { println!("max-lines = {n}  ({})", p.display()); 0 }
                Err(e) => { eprintln!("error: {e}"); 2 }
            },
            None => { eprintln!("usage: vtok config set max-lines N [--local]"); 2 }
        },
        Some("unset") if key_ok(w(1)) => match unset_max_lines(local) {
            Ok(p) => { println!("max-lines unset in {}", p.display()); 0 }
            Err(e) => { eprintln!("error: {e}"); 2 }
        },
        _ => {
            eprintln!("usage: vtok config [get max-lines | set max-lines N [--local] | unset max-lines [--local] | set mcp-run on|off]");
            2
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_beats_user_and_env_beats_both() {
        let d = std::env::temp_dir().join(format!("vtok-cfg-{}", std::process::id()));
        let sub = d.join("a/b");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(d.join(".vtok.json"), r#"{"max_lines": 77}"#).unwrap();
        let f = sub.join("x.rs");
        std::fs::write(&f, "fn x() {}\n").unwrap();
        assert_eq!(local_config_path(&f).unwrap(), d.join(".vtok.json"));
        assert_eq!(max_lines_in(&d.join(".vtok.json")), Some(77));
        std::fs::write(d.join(".vtok.json"), r#"{"max_lines": 0}"#).unwrap();
        assert_eq!(max_lines_in(&d.join(".vtok.json")), None);
        std::fs::remove_dir_all(&d).ok();
    }
}
