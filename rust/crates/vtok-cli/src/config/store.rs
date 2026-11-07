//! Writing the settings: `.vtok.json` in the working directory (`--local`) or the user config file.
use super::lookup::{read_json, user_config_path, KEY, MAX_LINES_LIMIT};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

fn target(local: bool) -> Result<PathBuf, String> {
    if local { Ok(std::env::current_dir().map_err(|e| e.to_string())?.join(".vtok.json")) }
    else { user_config_path().ok_or_else(|| "cannot locate the user config (set HOME or VTOK_CONFIG)".to_string()) }
}

fn write(path: &Path, v: &Value) -> Result<(), String> {
    if let Some(d) = path.parent() { std::fs::create_dir_all(d).map_err(|e| format!("{}: {e}", d.display()))?; }
    let text = serde_json::to_string_pretty(v).map_err(|e| e.to_string())? + "\n";
    std::fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))
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
