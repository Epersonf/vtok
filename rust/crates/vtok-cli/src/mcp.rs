//! Minimal MCP server (stdio, newline-delimited JSON-RPC 2.0) exposing Vtok to coding agents.
//!
//! `describe` is what the server announces (instructions, tool list); `args` is shared argument and file access; the
//! tools are `source` (one file), `project` (many files) and `command` (run, output). Here: dispatch and the stdio loop.
use serde_json::{json, Value};
use std::io::{BufRead, Write};

mod args;
mod command;
mod describe;
mod plain;
mod project;
mod source;
#[cfg(test)] mod t_plain;

use args::arg_str;
use command::{tool_output, tool_run};
use describe::{instructions, tools};
use project::{tool_find, tool_graph, tool_map, tool_refs};
use source::{tool_chunk, tool_edit, tool_outline, tool_read, tool_stats};

const PROTOCOL_FALLBACK: &str = "2025-06-18";

/// A panic inside one tool must not take the whole server (and the agent's connection) down.
fn call(name: &str, args: &Value) -> std::result::Result<String, String> {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| call_inner(name, args))) {
        Ok(r) => r,
        Err(_) => Err(format!("internal error in {name} (please report it); the server is still running")),
    }
}

fn call_inner(name: &str, args: &Value) -> std::result::Result<String, String> {
    match name {
        "vtok_read" => tool_read(args),
        "vtok_chunk" => tool_chunk(args),
        "vtok_edit" => tool_edit(args),
        "vtok_stats" => tool_stats(args),
        "vtok_outline" => tool_outline(args),
        "vtok_refs" => tool_refs(args),
        "vtok_map" => tool_map(args),
        "vtok_graph" => tool_graph(args),
        "vtok_route" => Ok(crate::route::classify(arg_str(args, "request")?, args.get("files").and_then(|v| v.as_u64()).unwrap_or(0) as usize).line() + "
"),
        "vtok_browse" => crate::browse::page(arg_str(args, "url")?, &crate::browse::Opts { budget: args.get("max_tokens").and_then(|v| v.as_u64()).unwrap_or(3000) as usize, links: args.get("links").and_then(|v| v.as_bool()).unwrap_or(true), allow_private: false }),
        "vtok_find" => tool_find(args),
        "vtok_run" => tool_run(args),
        "vtok_gain" => crate::usage::gain_text(args.get("days").and_then(|v| v.as_u64()).unwrap_or(30), false),
        "vtok_output" => tool_output(args),
        _ => Err(format!("unknown tool {name:?}")),
    }
}

fn handle(msg: &Value) -> Option<Value> {
    let method = msg.get("method")?.as_str()?;
    let id = msg.get("id").cloned();
    let params = msg.get("params").cloned().unwrap_or(Value::Null);
    let result = match method {
        "initialize" => {
            let v = params.get("protocolVersion").and_then(|v| v.as_str()).unwrap_or(PROTOCOL_FALLBACK);
            json!({"protocolVersion": v, "capabilities": {"tools": {}},
                   "serverInfo": {"name": "vtok", "version": env!("CARGO_PKG_VERSION")}, "instructions": instructions()})
        }
        "ping" => json!({}),
        "tools/list" => json!({"tools": tools()}),
        "tools/call" => {
            let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let args = params.get("arguments").cloned().unwrap_or(json!({}));
            match call(name, &args) {
                Ok(t) => json!({"content": [{"type": "text", "text": t}]}),
                Err(e) => json!({"content": [{"type": "text", "text": e}], "isError": true}),
            }
        }
        _ if method.starts_with("notifications/") => return None,
        _ => {
            return id.map(|id| json!({"jsonrpc": "2.0", "id": id, "error": {"code": -32601, "message": format!("method not found: {method}")}}));
        }
    };
    id.map(|id| json!({"jsonrpc": "2.0", "id": id, "result": result}))
}

pub fn serve(args: &[String]) -> i32 {
    if args.iter().any(|a| a == "--allow-run") { crate::config::ALLOW_RUN.store(true, std::sync::atomic::Ordering::Relaxed); }
    let stdin = std::io::stdin();
    let mut out = std::io::stdout();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() { continue; }
        let reply = match serde_json::from_str::<Value>(&line) {
            Ok(msg) => handle(&msg),
            Err(e) => Some(json!({"jsonrpc": "2.0", "id": Value::Null, "error": {"code": -32700, "message": format!("parse error: {e}")}})),
        };
        if let Some(r) = reply {
            let _ = writeln!(out, "{r}");
            let _ = out.flush();
        }
    }
    0
}
