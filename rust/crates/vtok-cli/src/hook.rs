//! `vtok hook`: a PreToolUse hook for agents that can rewrite a shell command before it runs (Claude Code does). It reads
//! the hook JSON on stdin and, for commands known to print a lot (tests, builds, installs, big listings), answers with the
//! same command wrapped in `vtok run --`, so the agent receives the compact view without having to remember to ask for it.
//! Anything it does not recognise passes through untouched (empty output). The permission decision is left to the user's
//! normal rules: the hook never grants approval by itself.
use serde_json::{json, Value};

const SHELL_META: &[char] = &['|', ';', '&', '<', '>', '`', '\n', '\\', '(', ')', '{', '}'];

fn words(cmd: &str) -> Vec<&str> {
    // skip leading VAR=value assignments
    cmd.split_whitespace().skip_while(|w| w.contains('=') && !w.starts_with('-') && !w.contains('/') || (w.contains('=') && w.chars().next().map_or(false, |c| c.is_ascii_uppercase()))).collect()
}

/// True for commands whose output is usually long and mostly noise.
pub fn is_chatty(cmd: &str) -> bool {
    let w = words(cmd);
    let Some(&prog) = w.first() else { return false };
    let prog = prog.rsplit('/').next().unwrap_or(prog);
    let sub = w.get(1).copied().unwrap_or("");
    let has = |x: &str| w.iter().skip(1).any(|a| *a == x);
    match prog {
        "pytest" | "py.test" | "jest" | "vitest" | "mocha" | "tsc" | "eslint" | "ruff" | "mypy" | "pylint" | "flake8" | "make" | "mvn" | "gradle" | "gradlew" | "tox" | "nox" | "rspec" | "phpunit" => true,
        "python" | "python3" => sub == "-m" && matches!(w.get(2).copied().unwrap_or(""), "pytest" | "unittest" | "pip" | "mypy" | "ruff" | "tox"),
        "cargo" => matches!(sub, "test" | "build" | "check" | "clippy" | "doc" | "bench" | "install" | "update" | "fetch"),
        "npm" | "pnpm" | "yarn" | "bun" => matches!(sub, "test" | "install" | "i" | "ci" | "add" | "build" | "lint" | "audit") || (sub == "run" && matches!(w.get(2).copied().unwrap_or(""), "test" | "build" | "lint" | "typecheck")),
        "pip" | "pip3" | "uv" | "poetry" => matches!(sub, "install" | "list" | "freeze" | "sync" | "lock" | "add") || (sub == "pip" && matches!(w.get(2).copied().unwrap_or(""), "install" | "list" | "freeze")),
        "go" => matches!(sub, "test" | "build" | "vet" | "mod" | "get"),
        "dotnet" => matches!(sub, "test" | "build" | "restore"),
        "docker" | "podman" => matches!(sub, "build" | "logs" | "pull" | "ps" | "images"),
        "kubectl" => matches!(sub, "logs" | "get" | "describe"),
        "git" => matches!(sub, "log" | "diff" | "show" | "status" | "blame") && !has("--oneline") && !has("-1") && !has("-n1") && !w.iter().any(|a| a.starts_with("--format") || a.starts_with("--pretty") || a.starts_with("--stat=") && false || *a == "--quiet" || *a == "-q"),
        "find" | "tree" => true,
        "ls" => w.iter().skip(1).any(|a| a.starts_with('-') && a.contains('R')),
        "grep" | "rg" => w.iter().skip(1).any(|a| a.starts_with('-') && (a.contains('r') || a.contains('R'))) || prog == "rg",
        _ => false,
    }
}

/// The rewritten command, or None to leave the original alone.
/// Split a leading `cd DIR && ` / `export X=y && ` prefix (agents add it to nearly every command) from the command itself.
pub fn split_prefix(cmd: &str) -> (&str, &str) {
    let mut rest = cmd.trim_start();
    let mut used = 0;
    while let Some(i) = rest.find("&&") {
        let head = rest[..i].trim();
        let simple = |h: &str| !h.is_empty() && !h.contains(['|', ';', '<', '>', '`', '\n', '(', ')', '{', '}', '&']) && !h.contains("$(");
        if (head.starts_with("cd ") || head.starts_with("export ")) && simple(head) {
            used += cmd.trim_start().len() - rest.len() + i + 2;
            rest = &rest[i + 2..];
        } else { break; }
    }
    let off = cmd.len() - cmd.trim_start().len() + used;
    (&cmd[..off.min(cmd.len())], cmd[off.min(cmd.len())..].trim_start())
}

/// The rewritten command, or None to leave the original alone.
pub fn rewrite(cmd: &str) -> Option<String> {
    let (pre, t) = split_prefix(cmd);
    let t = t.trim();
    if t.is_empty() || t.starts_with("vtok ") || t.contains(SHELL_META) || t.contains("$(") { return None; }
    if !is_chatty(t) { return None; }
    Some(format!("{}vtok run -- {t}", if pre.is_empty() { String::new() } else { format!("{} ", pre.trim()) }))
}

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
    fn wraps_chatty_commands_only() {
        assert_eq!(rewrite("cargo test --release").as_deref(), Some("vtok run -- cargo test --release"));
        assert_eq!(rewrite("FOO=1 python -m pytest -v tests").as_deref(), Some("vtok run -- FOO=1 python -m pytest -v tests"));
        assert!(rewrite("npm install").is_some() && rewrite("git log --stat").is_some() && rewrite("grep -rn foo src").is_some());
        for no in ["ls", "cat a.txt", "git log --oneline", "git status -q", "cargo test | head", "vtok run -- cargo test", "python script.py", "cd x\ncargo test", "grep foo file", "cargo run"] {
            assert!(rewrite(no).is_none(), "{no}");
        }
    }

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
