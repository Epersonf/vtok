//! `vtok hook --agent NAME`: the PreToolUse / BeforeTool hook protocol of each supported agent. Agents whose protocol
//! needs an explicit "allow" to apply a rewrite (cursor, codex, gemini, antigravity) only get low-risk commands rewritten
//! (see risk.rs); the others (claude, copilot, droid, trae, vibe) apply the rewrite without granting any permission.
use serde_json::{json, Value};

fn needs_allow(agent: &str) -> bool { matches!(agent, "cursor" | "codex" | "gemini" | "antigravity") }

fn command_of(agent: &str, v: &Value) -> Option<String> {
    let p = if agent == "antigravity" { v.pointer("/toolCall/args/CommandLine").or_else(|| v.pointer("/toolCall/args/command")) } else { v.pointer("/tool_input/command") };
    p.and_then(|c| c.as_str()).map(String::from)
}

/// The hook's stdout for `input`, or None to stay silent (the command runs as typed).
pub fn answer(agent: &str, input: &str) -> Option<String> {
    let v: Value = serde_json::from_str(input).ok()?;
    let cmd = command_of(agent, &v)?;
    let new = crate::rewrite::rewrite_with(&cmd, needs_allow(agent));
    if new.is_none() && agent != "gemini" && agent != "antigravity" { return None; }
    let out = match (agent, new) {
        ("cursor", Some(n)) => json!({"continue": true, "permission": "allow", "updated_input": {"command": n}}),
        ("gemini", Some(n)) => json!({"decision": "allow", "hookSpecificOutput": {"tool_input": {"command": n}}}),
        ("gemini", None) | ("antigravity", None) => json!({"decision": "allow"}),
        ("antigravity", Some(n)) => json!({"decision": "allow", "reason": "Rewritten by vtok to save tokens.", "overwrite": {"CommandLine": n}}),
        ("vibe", Some(n)) => json!({"hook_specific_output": {"tool_input": {"command": n}}, "system_message": "vtok: long output is compacted (vtok output [ID] shows the rest; no ID = the last run)"}),
        (a, Some(n)) => {
            let mut ti = v.get("tool_input")?.clone();
            ti["command"] = json!(n);
            let mut h = json!({"hookEventName": "PreToolUse", "updatedInput": ti});
            if a == "codex" { h["permissionDecision"] = json!("allow"); }
            json!({"hookSpecificOutput": h})
        }
        (_, None) => return None,
    };
    Some(out.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call(agent: &str, cmd: &str) -> Option<Value> {
        answer(agent, &json!({"tool_name": "x", "tool_input": {"command": cmd}, "toolCall": {"args": {"CommandLine": cmd}}}).to_string()).map(|s| serde_json::from_str(&s).unwrap())
    }

    #[test]
    fn each_protocol_shape() {
        assert_eq!(call("droid", "cargo test").unwrap()["hookSpecificOutput"]["updatedInput"]["command"], "vtok run -- cargo test");
        assert!(call("droid", "cargo test").unwrap()["hookSpecificOutput"].get("permissionDecision").is_none());
        assert_eq!(call("cursor", "cargo test").unwrap()["updated_input"]["command"], "vtok run -- cargo test");
        assert_eq!(call("codex", "pytest").unwrap()["hookSpecificOutput"]["permissionDecision"], "allow");
        assert_eq!(call("gemini", "git status").unwrap()["hookSpecificOutput"]["tool_input"]["command"], "vtok run -- git status");
        assert_eq!(call("antigravity", "go test ./...").unwrap()["overwrite"]["CommandLine"], "vtok run -- go test ./...");
        assert_eq!(call("vibe", "npm install").unwrap()["hook_specific_output"]["tool_input"]["command"], "vtok run -- npm install");
    }

    #[test]
    fn allow_agents_skip_risky_commands() {
        for a in ["cursor", "codex"] { assert!(call(a, "npm install").is_none() && call(a, "git push").is_none(), "{a}"); }
        assert_eq!(call("gemini", "npm install").unwrap(), json!({"decision": "allow"}));
        assert!(call("claude", "npm install").is_some());
        assert!(call("claude", "echo hi").is_none());
    }
}
