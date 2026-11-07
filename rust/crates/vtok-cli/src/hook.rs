//! `vtok hook`: a PreToolUse hook for agents that can rewrite a shell command before it runs (Claude Code does). It reads
//! the hook JSON on stdin and, for commands known to print a lot (tests, builds, installs, big listings), answers with the
//! same command wrapped in `vtok run --`, so the agent receives the compact view without having to remember to ask for it.
//! Anything it does not recognise passes through untouched (empty output). The permission decision is left to the user's
//! normal rules: the hook never grants approval by itself.
use serde_json::{json, Value};

use crate::rewrite::rewrite;

pub fn answer(input: &str) -> Option<String> {
    let v: Value = serde_json::from_str(input).ok()?;
    if v.get("tool_name").and_then(|x| x.as_str()) != Some("Bash") { return None; }
    let mut ti = v.get("tool_input")?.clone();
    let cmd = ti.get("command")?.as_str()?.to_string();
    let new = rewrite(&cmd)?;
    ti["command"] = json!(new);
    Some(json!({"hookSpecificOutput": {"hookEventName": "PreToolUse", "updatedInput": ti}}).to_string())
}

pub fn run(_args: &[String]) -> i32 {
    use std::io::Read;
    let mut s = String::new();
    if std::io::stdin().read_to_string(&mut s).is_err() { return 0; }
    if let Some(a) = answer(&s) { println!("{a}"); }
    0
}

/// Add the hook to a Claude Code settings file (idempotent), keeping everything else in it.
pub fn install(path: &str) -> Result<bool, String> {
    let mut v: Value = match std::fs::read_to_string(path) {
        Ok(t) if !t.trim().is_empty() => serde_json::from_str(&t).map_err(|e| format!("{path}: not valid JSON ({e}); left untouched"))?,
        _ => json!({}),
    };
    if !v.is_object() { return Err(format!("{path}: not a JSON object; left untouched")); }
    let arr = v.as_object_mut().unwrap().entry("hooks").or_insert_with(|| json!({}))
        .as_object_mut().ok_or("hooks is not an object")?.entry("PreToolUse").or_insert_with(|| json!([]))
        .as_array_mut().ok_or("PreToolUse is not an array")?;
    if arr.iter().any(|e| e.to_string().contains("vtok hook")) { return Ok(false); }
    arr.push(json!({"matcher": "Bash", "hooks": [{"type": "command", "command": "vtok hook"}]}));
    if let Some(d) = std::path::Path::new(path).parent() { if !d.as_os_str().is_empty() { std::fs::create_dir_all(d).map_err(|e| e.to_string())?; } }
    std::fs::write(path, serde_json::to_string_pretty(&v).unwrap() + "\n").map_err(|e| format!("{path}: {e}"))?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn answer_keeps_other_fields_and_ignores_other_tools() {
        let a = answer(r#"{"tool_name":"Bash","tool_input":{"command":"pytest -v","timeout":5000}}"#).unwrap();
        let v: Value = serde_json::from_str(&a).unwrap();
        assert_eq!(v["hookSpecificOutput"]["updatedInput"]["command"], "vtok run -- pytest -v");
        assert_eq!(v["hookSpecificOutput"]["updatedInput"]["timeout"], 5000);
        assert!(v["hookSpecificOutput"].get("permissionDecision").is_none());
        assert!(answer(r#"{"tool_name":"Read","tool_input":{"file_path":"x"}}"#).is_none());
        assert!(answer("not json").is_none());
    }

    #[test]
    fn install_is_idempotent_and_keeps_settings() {
        let p = std::env::temp_dir().join(format!("vtok-hook-{}.json", std::process::id()));
        std::fs::write(&p, r#"{"model":"x","hooks":{"PreToolUse":[{"matcher":"Edit","hooks":[]}]}}"#).unwrap();
        let ps = p.to_str().unwrap();
        assert!(install(ps).unwrap());
        assert!(!install(ps).unwrap());
        let v: Value = serde_json::from_str(&std::fs::read_to_string(&p).unwrap()).unwrap();
        assert_eq!(v["model"], "x");
        assert_eq!(v["hooks"]["PreToolUse"].as_array().unwrap().len(), 2);
        std::fs::remove_file(&p).ok();
    }
}
