//! `vtok init --hook AGENT [--global]`: register the vtok hook (or write the plugin file) for an agent. Every step is
//! idempotent and keeps the rest of the agent's configuration; a file that is not valid JSON is left untouched.
use crate::hook_plugins as p;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

pub const NAMES: &str = "claude, copilot, droid, trae, vibe, cursor, codex, gemini, antigravity, opencode, pi, omp, hermes";

fn home() -> PathBuf { std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(PathBuf::from).unwrap_or_else(|| PathBuf::from(".")) }
fn base(global: bool, dir: &str) -> PathBuf { if global { home().join(dir) } else { PathBuf::from(dir) } }

fn write(path: &Path, text: &str) -> Result<(), String> {
    if let Some(d) = path.parent() { std::fs::create_dir_all(d).map_err(|e| format!("{}: {e}", d.display()))?; }
    std::fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))
}

/// Add `entry` under hooks.EVENT in a JSON config; Ok(false) when a vtok entry is already there.
fn add_entry(path: &Path, root: Value, event: &str, entry: Value) -> Result<bool, String> {
    let mut v = match std::fs::read_to_string(path) {
        Ok(t) if !t.trim().is_empty() => serde_json::from_str::<Value>(&t).map_err(|e| format!("{}: not valid JSON ({e}); left untouched", path.display()))?,
        _ => root,
    };
    let arr = v.as_object_mut().ok_or("config is not a JSON object")?.entry("hooks").or_insert_with(|| json!({}))
        .as_object_mut().ok_or("hooks is not an object")?.entry(event).or_insert_with(|| json!([]))
        .as_array_mut().ok_or("hook list is not an array")?;
    if arr.iter().any(|e| e.to_string().contains("vtok hook")) { return Ok(false); }
    arr.push(entry);
    write(path, &(serde_json::to_string_pretty(&v).unwrap() + "\n"))?;
    Ok(true)
}

fn cmd_entry(matcher: &str, agent: &str) -> Value { json!({"matcher": matcher, "hooks": [{"type": "command", "command": format!("vtok hook --agent {agent}"), "timeout": 30}]}) }

/// Install for `agent`; returns a line describing what changed.
pub fn install(agent: &str, global: bool) -> Result<String, String> {
    let done = |changed: bool, path: &Path| format!("{}: {}", path.display(), if changed { "vtok hook added" } else { "already up to date" });
    let json_hook = |path: PathBuf, root: Value, event: &str, entry: Value| add_entry(&path, root, event, entry).map(|c| done(c, &path));
    let file = |path: PathBuf, text: &str| -> Result<String, String> {
        let same = std::fs::read_to_string(&path).map_or(false, |t| t == text);
        if !same { write(&path, text)?; }
        Ok(done(!same, &path))
    };
    match agent {
        "claude" => { let path = base(global, ".claude").join("settings.json"); crate::hook::install(&path.to_string_lossy()).map(|c| done(c, &path)) }
        "droid" => json_hook(base(global, ".factory").join("hooks.json"), json!({}), "PreToolUse", cmd_entry("Execute", "droid")),
        "trae" => json_hook(base(global, ".trae").join("hooks.json"), json!({"version": 1}), "PreToolUse", cmd_entry("RunCommand", "trae")),
        "codex" => { let d = std::env::var_os("CODEX_HOME").map(PathBuf::from).filter(|_| global).unwrap_or_else(|| base(global, ".codex")); json_hook(d.join("hooks.json"), json!({}), "PreToolUse", cmd_entry("Bash", "codex")) }
        "gemini" => json_hook(base(global, ".gemini").join("settings.json"), json!({}), "BeforeTool", cmd_entry("run_shell_command", "gemini")),
        "cursor" => json_hook(base(global, ".cursor").join("hooks.json"), json!({"version": 1}), "preToolUse", json!({"matcher": "Shell", "command": "vtok hook --agent cursor"})),
        "copilot" => json_hook(PathBuf::from(".github/hooks/vtok-rewrite.json"), json!({"version": 1}), "PreToolUse", json!({"type": "command", "command": "vtok hook --agent copilot", "cwd": ".", "timeout": 5})),
        "antigravity" => {
            let d = if global { home().join(".gemini/config/plugins/vtok") } else { PathBuf::from(".agents/plugins/vtok") };
            file(d.join("plugin.json"), p::ANTIGRAVITY_PLUGIN)?;
            file(d.join("hooks.json"), p::ANTIGRAVITY_HOOKS)
        }
        "vibe" => {
            let path = home().join(".vibe/hooks.toml");
            let old = std::fs::read_to_string(&path).unwrap_or_default();
            if old.contains("name = \"vtok-rewrite\"") { return Ok(done(false, &path)); }
            let sep = if old.is_empty() || old.ends_with("\n\n") { "" } else if old.ends_with('\n') { "\n" } else { "\n\n" };
            write(&path, &format!("{old}{sep}{}", p::VIBE_ENTRY))?;
            Ok(done(true, &path))
        }
        "opencode" => file(home().join(".config/opencode/plugins/vtok.ts"), p::OPENCODE),
        "pi" => file(if global { home().join(".pi/agent/extensions/vtok.ts") } else { PathBuf::from(".pi/extensions/vtok.ts") }, p::PI),
        "omp" => file(if global { home().join(".omp/agent/extensions/vtok.ts") } else { PathBuf::from(".omp/extensions/vtok.ts") }, p::PI),
        "hermes" => { let d = home().join(".hermes/plugins/vtok-rewrite"); file(d.join("__init__.py"), p::HERMES_INIT)?; file(d.join("plugin.yaml"), p::HERMES_YAML) }
        other => Err(format!("unknown agent {other:?}; known: {NAMES}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_registration_is_idempotent_and_keeps_other_hooks() {
        let d = std::env::temp_dir().join(format!("vtok-hooks-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let path = d.join("hooks.json");
        std::fs::write(&path, r#"{"version":1,"hooks":{"preToolUse":[{"matcher":"Other","command":"x"}]}}"#).unwrap();
        let e = json!({"matcher": "Shell", "command": "vtok hook --agent cursor"});
        assert!(add_entry(&path, json!({}), "preToolUse", e.clone()).unwrap());
        assert!(!add_entry(&path, json!({}), "preToolUse", e).unwrap());
        let v: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(v["hooks"]["preToolUse"].as_array().unwrap().len(), 2);
        std::fs::write(&path, "not json").unwrap();
        assert!(add_entry(&path, json!({}), "preToolUse", json!({})).is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "not json");
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn unknown_agent_lists_the_known_ones() {
        assert!(install("nope", false).unwrap_err().contains("claude, copilot"));
    }
}
