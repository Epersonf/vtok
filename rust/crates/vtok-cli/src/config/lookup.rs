//! Reading the settings: where the user and repo config files are, the effective max-lines, whether the MCP server may run commands.
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

pub static ALLOW_RUN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub const DEFAULT_MAX_LINES: usize = 150;
pub const MAX_LINES_LIMIT: usize = 100_000;
pub(super) const KEY: &str = "max_lines";

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

pub(super) fn read_json(path: &Path) -> Value {
    std::fs::read_to_string(path).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_else(|| json!({}))
}

pub(super) fn max_lines_in(path: &Path) -> Option<usize> {
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

/// Whether the MCP server may run commands (`vtok_run`). Off by default. Only the command line (`vtok mcp
/// --allow-run`), `VTOK_MCP_RUN=1`, or the USER config can turn it on - never a repo-local `.vtok.json`, so a cloned
/// repository cannot grant itself command execution.
pub fn mcp_run_enabled() -> bool {
    ALLOW_RUN.load(std::sync::atomic::Ordering::Relaxed)
        || std::env::var("VTOK_MCP_RUN").map_or(false, |v| v == "1")
        || user_config_path().map_or(false, |p| read_json(&p).get("mcp_run").and_then(|v| v.as_bool()) == Some(true))
}
