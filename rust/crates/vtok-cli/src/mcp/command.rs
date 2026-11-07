//! Tools that run a command or read the stored output of one: run, output.
use super::args::arg_str;
use serde_json::Value;
use vtok_api::ntok;

pub(super) fn tool_run(a: &Value) -> std::result::Result<String, String> {
    if !crate::config::mcp_run_enabled() {
        return Err("vtok_run is disabled. The user can enable it with `vtok config set mcp-run on` or by starting the server with `vtok mcp --allow-run`; until then run the command with your shell tool as `vtok run -- <command>`.".into());
    }
    let command = arg_str(a, "command")?;
    let cwd = a.get("cwd").and_then(|v| v.as_str());
    let timeout = a.get("timeout_s").and_then(|v| v.as_u64()).unwrap_or(120).min(3600);
    let budget = a.get("max_tokens").and_then(|v| v.as_u64()).map_or(700, |n| n.clamp(100, 8000) as usize);
    crate::shell::run_for_mcp(command, cwd, timeout, budget)
}

pub(super) fn tool_output(a: &Value) -> std::result::Result<String, String> {
    let id = a.get("id").and_then(|v| v.as_str()).filter(|s| !s.is_empty());
    let num = |k: &str| a.get(k).and_then(|v| v.as_u64()).map(|n| n as usize);
    let q = crate::shell::Query {
        grep: a.get("grep").and_then(|v| v.as_str()).map(|s| s.split('|').map(String::from).collect()).unwrap_or_default(),
        range: a.get("lines").and_then(|v| v.as_str()).and_then(crate::shell::parse_range),
        tail: num("tail"),
        head: num("head"),
        context: num("context").unwrap_or(2),
        max_tokens: num("max_tokens").unwrap_or(2000),
    };
    let out = crate::shell::fetch(id, &q)?;
    crate::usage::record("output", 0, ntok(&out), id.unwrap_or("latest"));
    Ok(out)
}
