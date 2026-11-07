//! Minimal MCP server (stdio, newline-delimited JSON-RPC 2.0) exposing Vtok to coding agents.
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::io::{BufRead, Write};
use vtok_api::*;

const PROTOCOL_FALLBACK: &str = "2025-06-18";

fn fnv(s: &str) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.as_bytes() { h ^= *b as u64; h = h.wrapping_mul(0x100000001b3); }
    format!("{h:016x}")
}

fn instructions() -> String {
    let mut s = String::from("Vtok shows source code in a compact notation that costs fewer tokens (comments and docstrings are dropped, names are aliased, whitespace is minimal). Use vtok_read instead of reading Python / TypeScript / JavaScript / Rust files when you do not need the comments; it prints an alias table (alias=original) followed by the code. Cheapest way to work on a big file: vtok_outline lists the name and line range of every symbol (a few hundred tokens); to locate something in a project use vtok_find with the words of your question (show=1 returns the compact code of the best match in the same call) or vtok_refs for where a name is defined and used; then vtok_chunk with id = one of those names (`name`, or `Class.method` when ambiguous) returns just that item, with the aliases it uses. If you already know the name, call vtok_chunk directly. vtok_read with view=skeleton adds signatures but costs about 3x more than the outline. Big files read in full come back as an index plus a chunk map (ids like c3), which vtok_chunk also accepts. To change code, send the edited Vtok of a chunk or symbol to vtok_edit with the rev from the read (the server decodes it, splices it into the original file and rejects the edit if the file changed). Only the tokens you change are rewritten; comments next to a changed token are kept, comments inside a replaced item are lost; for one-line tweaks the normal edit tool may be better. New names are written in full; names in the alias table stay aliased.\n\nUse these tools only when they save tokens: read a small file (under ~100 lines) or run a short command directly; use a narrow pattern for vtok_output (matches are capped); if a Vtok call would cost more than the plain alternative, skip it.\n\nLong command output: use vtok_run if it is available (it is only listed when the user enabled it); otherwise run the command through your shell tool as `vtok run -- <command>`. Either way you get a compact view (errors, summary, head/tail, repeated lines collapsed). The full output is stored: when the view cut something or the command failed, its last line names an id; vtok_output(grep/lines/tail) returns any part of it, and without an id it reads the most recent run (e.g. the warnings behind a count such as `4 warnings`).\n");
    for l in languages() {
        s.push_str(&format!("\n[{}]\n{}", l.name(), l.grammar_prompt()));
    }
    s
}

fn tools() -> Value {
    let all = json!([
      {"name": "vtok_read",
       "description": "Compact Vtok of a Python/TypeScript/JavaScript/Rust/Java/Go/C/C++/C#/Dart/CSS/HTML/SQL file: rev, alias table, code. Big files return an index + chunk map. view: full, skeleton or lean.",
       "inputSchema": {"type": "object", "properties": {
          "path": {"type": "string"}, "view": {"type": "string", "enum": ["full", "skeleton", "lean"]},
          "max_lines": {"type": "integer", "description": "split files longer than this (default: `vtok config` max-lines, else 150)"}}, "required": ["path"]}},
      {"name": "vtok_chunk",
       "description": "Compact code of one symbol (name, or Class.method) or chunk id (c3), with its rev and aliases.",
       "inputSchema": {"type": "object", "properties": {"path": {"type": "string"}, "id": {"type": "string"}, "max_lines": {"type": "integer"}}, "required": ["path", "id"]}},
      {"name": "vtok_edit",
       "description": "Replace symbols/chunks with edited Vtok: chunks maps id -> complete new Vtok; rev from the read/chunk. Untouched tokens keep their original text.",
       "inputSchema": {"type": "object", "properties": {
          "path": {"type": "string"}, "rev": {"type": "string"},
          "chunks": {"type": "object", "additionalProperties": {"type": "string"}},
          "max_lines": {"type": "integer"}, "dry_run": {"type": "boolean"}}, "required": ["path", "rev", "chunks"]}},
      {"name": "vtok_outline",
       "description": "Symbol names and line ranges of a file: the cheapest overview.",
       "inputSchema": {"type": "object", "properties": {"path": {"type": "string"}}, "required": ["path"]}},
      {"name": "vtok_refs",
       "description": "Where a symbol is defined and used across a project, one line each with file and line range (by name, not type).",
       "inputSchema": {"type": "object", "properties": {"name": {"type": "string"}, "path": {"type": "string", "description": "directory to search (default: server working directory)"}, "max_tokens": {"type": "integer", "description": "default 500"}}, "required": ["name"]}},
      {"name": "vtok_find",
       "description": "Find symbols from the words of a question (matches names, path, signature words): file, symbol, line range. show=N also returns the compact code of the top N matches.",
       "inputSchema": {"type": "object", "properties": {"query": {"type": "string"}, "path": {"type": "string", "description": "directory (default: server working directory)"}, "max_tokens": {"type": "integer", "description": "default 400"}, "show": {"type": "integer", "description": "also return the compact code (with rev and aliases, ready for vtok_edit) of the top N matches, 0-3, default 0"}}, "required": ["query"]}},
      {"name": "vtok_map",
       "description": "Overview of a project directory within a token budget: every Python/TypeScript/JavaScript/Rust/Java/Go/C/C++/C#/Dart/CSS/HTML/SQL file with its size and its most used symbols (line range, x N = uses in other files, ~ = several definitions share the name), plus signatures for the top few. Use it to decide where to look; then vtok_outline / vtok_chunk. A plain file listing is cheaper when file names already tell you where to look.",
       "inputSchema": {"type": "object", "properties": {"path": {"type": "string", "description": "directory (default: server working directory)"}, "max_tokens": {"type": "integer", "description": "default 700"}}}},
      {"name": "vtok_browse",
       "description": "Fetch a web page (http/https, public hosts only) and return it as compact text: headings, lists, tables, code and links kept, scripts/styles/navigation dropped. The full text stays stored under the returned id (vtok_output grep). Use it instead of fetching raw HTML.",
       "inputSchema": {"type": "object", "properties": {"url": {"type": "string"}, "max_tokens": {"type": "integer", "description": "default 3000"}, "links": {"type": "boolean", "description": "keep link URLs (default true)"}}, "required": ["url"]}},
      {"name": "vtok_route",
       "description": "Which model tier and reasoning effort fits a request (small / medium / large, with the configured model name): a cheap deterministic score of the request's size, keywords, code and files. Use it to choose the model for a subagent or a delegated task.",
       "inputSchema": {"type": "object", "properties": {"request": {"type": "string"}, "files": {"type": "integer", "description": "number of files involved, if known"}}, "required": ["request"]}},
      {"name": "vtok_graph",
       "description": "Relation graph of the project (scope-aware): action explain (a symbol's relations grouped by kind: calls, uses_type, references, implements, extends, contains, imports, tests; a score is shown when below 1.0), path (shortest chain between two symbols), hubs (most connected), communities (groups of related symbols). Built once into .vtok/graph.json and refreshed when sources change.",
       "inputSchema": {"type": "object", "properties": {"action": {"type": "string", "description": "explain | path | hubs | communities"}, "a": {"type": "string", "description": "symbol name (explain, path start) or count (hubs, communities)"}, "b": {"type": "string", "description": "path end symbol"}, "path": {"type": "string", "description": "project directory (default: server working directory)"}}, "required": ["action"]}},
      {"name": "vtok_output",
       "description": "Read part of a stored `vtok run` / vtok_run output: grep, lines A-B, tail or head. id is the one a result's last line names; without it, the most recent run is read.",
       "inputSchema": {"type": "object", "properties": {"id": {"type": "string", "description": "optional: default is the most recent run"}, "grep": {"type": "string"}, "lines": {"type": "string", "description": "A-B (1-based)"}, "tail": {"type": "integer"}, "head": {"type": "integer"}, "context": {"type": "integer", "description": "lines around each grep hit (default 2)"}, "max_tokens": {"type": "integer"}}}},
      {"name": "vtok_run",
       "description": "Run a shell command and get a compact view of its output (errors, summary, head/tail, repeated lines collapsed); the full output stays stored and vtok_output(grep/lines/tail) returns any part of the most recent run. Use it instead of a plain shell call for commands with long output (test runs, builds, installs, listings). The command runs with the user's privileges, in cwd (default: the server's directory), and is killed after timeout_s (default 120, max 900). A failed or timed-out command starts with `[exit N]`; a result whose view cut something ends with `# more: vtok output ID`; a successful, fully summarised run is just the summary.",
       "inputSchema": {"type": "object", "properties": {"command": {"type": "string"}, "cwd": {"type": "string"}, "timeout_s": {"type": "integer"}, "max_tokens": {"type": "integer", "description": "size of the compact view (default 700)"}}, "required": ["command"]}},
      {"name": "vtok_gain",
       "description": "Tokens saved by Vtok's tools on this machine (local usage log): calls, raw vs shown tokens per tool. Upper bound: raw means the whole file or whole command output.",
       "inputSchema": {"type": "object", "properties": {"days": {"type": "integer", "description": "default 30"}}}},
      {"name": "vtok_stats",
       "description": "Token counts of files: raw vs Vtok (incl. alias table). Use it to decide whether vtok_read pays off.",
       "inputSchema": {"type": "object", "properties": {"paths": {"type": "array", "items": {"type": "string"}}}, "required": ["paths"]}}
    ]);
    let mut v = all;
    if let Some(a) = v.as_array_mut() { if !crate::config::mcp_run_enabled() { a.retain(|t| t["name"] != "vtok_run"); }
        // fewer tool schemas = fewer tokens on every turn; these stay available from the command line (and as MCP calls)
        if std::env::var("VTOK_MCP_ALL").map_or(true, |v| v != "1") { a.retain(|t| !matches!(t["name"].as_str(), Some("vtok_gain") | Some("vtok_stats") | Some("vtok_map") | Some("vtok_graph") | Some("vtok_browse"))); } }
    v
}

fn read_source(path: &str) -> std::result::Result<(String, &'static dyn Language, bool), String> {
    let lang = language_for_path(path).map_err(|e| format!("{e} (supported: {}; use the normal read tool for this file)",languages().iter().flat_map(|l|l.extensions().iter().copied()).collect::<Vec<_>>().join(" ")))?;
    let raw = std::fs::read_to_string(path).map_err(|e| format!("cannot read {path}: {e}"))?;
    let crlf = raw.contains("\r\n");
    Ok((normalize_newlines(&raw), lang, crlf))
}

fn arg_str<'a>(a: &'a Value, k: &str) -> std::result::Result<&'a str, String> {
    a.get(k).and_then(|v| v.as_str()).ok_or_else(|| format!("missing string argument {k:?}"))
}

fn max_lines(a: &Value) -> usize {
    a.get("max_lines").and_then(|v| v.as_u64()).map(|n| n.max(1) as usize).unwrap_or_else(|| {
        crate::config::max_lines(a.get("path").and_then(|v| v.as_str()).map(std::path::Path::new)).0
    })
}

fn tool_read(a: &Value) -> std::result::Result<String, String> {
    let path = arg_str(a, "path")?;
    let (src, lang, _) = read_source(path)?;
    let view = a.get("view").and_then(|v| v.as_str()).unwrap_or("full");
    let rev = fnv(&src);
    let raw_tokens = ntok(&src);
    let (body, table, note) = if view == "full" {
        let s = split_file(lang.name(), &src, max_lines(a)).map_err(|e| e.to_string())?;
        let n = if s.whole_file { format!("1 chunk (c1 = whole file), {} lines", src.lines().count()) } else { format!("{} chunks, {} lines", s.chunks.len(), src.lines().count()) };
        (s.index, s.table, n)
    } else {
        let o = Options { view: Some(view.to_string()), ..Default::default() };
        let r = encode(lang.name(), &src, &o).map_err(|e| e.to_string())?;
        (r.vtok, r.table.clone(), format!("{view} view (read-only)"))
    };
    let tbl = render_table(&table);
    let total = ntok(&body) + ntok(&tbl);
    if view == "full" && (total + 30) * 10 >= raw_tokens * 9 {
        crate::usage::record("read", raw_tokens, raw_tokens, path);
        return Ok(format!("# {path} rev={rev}: Vtok would not be smaller ({total} vs {raw_tokens} tokens); raw file below. Edit it with the normal edit tool.\n---\n{src}"));
    }
    crate::usage::record("read", raw_tokens, total, path);
    Ok(format!("# {path} lang={} rev={rev} {note}; {raw_tokens} raw tokens -> {total}\n# aliases (alias=original)\n{tbl}\n---\n{body}", lang.name()))
}

fn tool_chunk(a: &Value) -> std::result::Result<String, String> {
    let path = arg_str(a, "path")?;
    let id = arg_str(a, "id")?;
    let (src, lang, _) = read_source(path)?;
    let (v, used) = chunk_with_aliases(lang.name(), &src, id, max_lines(a)).map_err(|e| e.to_string())?;
    let aliases = if used.is_empty() { String::new() } else { format!("# aliases used (alias=original)\n{}\n", render_table(&used)) };
    let out = format!("# {path} chunk {id} rev={}\n{aliases}---\n{v}", fnv(&src));
    crate::usage::record("chunk", ntok(&src), ntok(&out), &format!("{path} {id}"));
    Ok(out)
}

fn tool_run(a: &Value) -> std::result::Result<String, String> {
    if !crate::config::mcp_run_enabled() {
        return Err("vtok_run is disabled. The user can enable it with `vtok config set mcp-run on` or by starting the server with `vtok mcp --allow-run`; until then run the command with your shell tool as `vtok run -- <command>`.".into());
    }
    let command = arg_str(a, "command")?;
    let cwd = a.get("cwd").and_then(|v| v.as_str());
    let timeout = a.get("timeout_s").and_then(|v| v.as_u64()).unwrap_or(120).min(3600);
    let budget = a.get("max_tokens").and_then(|v| v.as_u64()).map_or(700, |n| n.clamp(100, 8000) as usize);
    crate::shell::run_for_mcp(command, cwd, timeout, budget)
}

fn tool_refs(a: &Value) -> std::result::Result<String, String> {
    let name = arg_str(a, "name")?;
    if name.is_empty() || !name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '$') { return Err("name must be a single identifier".into()); }
    let dir = a.get("path").and_then(|v| v.as_str()).unwrap_or(".");
    let budget = a.get("max_tokens").and_then(|v| v.as_u64()).map_or(500, |n| n.clamp(100, 8000) as usize);
    let r = crate::refs::find(name, std::path::Path::new(dir), budget);
    crate::usage::record("refs", r.raw_tokens.max(ntok(&r.text)), ntok(&r.text), name);
    Ok(r.text)
}

fn tool_find(a: &Value) -> std::result::Result<String, String> {
    let q = arg_str(a, "query")?;
    let dir = a.get("path").and_then(|v| v.as_str()).unwrap_or(".");
    let budget = a.get("max_tokens").and_then(|v| v.as_u64()).map_or(400, |n| n.clamp(100, 8000) as usize);
    let show = a.get("show").and_then(|v| v.as_u64()).unwrap_or(0).min(3) as usize;
    let r = crate::refs::find_symbols(q, std::path::Path::new(dir), budget);
    let mut out = r.text.clone();
    // one call instead of find + chunk: the compact code of the best matches
    for (rel, id) in r.top.iter().take(show) {
        let full = std::path::Path::new(dir).join(rel);
        match tool_chunk(&json!({"path": full.to_string_lossy(), "id": id})) {
            Ok(c) if ntok(&c) <= 700 => { out.push('\n'); out.push_str(&c); }
            Ok(c) => out.push_str(&format!("\n# {rel} {id}: {} tokens, too long to inline; use vtok_chunk\n", ntok(&c))),
            Err(e) => out.push_str(&format!("\n# {rel} {id}: {e}\n")),
        }
    }
    crate::usage::record("find", r.raw_tokens.max(ntok(&out)), ntok(&out), q);
    Ok(out)
}

fn tool_map(a: &Value) -> std::result::Result<String, String> {
    let dir = a.get("path").and_then(|v| v.as_str()).unwrap_or(".");
    let budget = a.get("max_tokens").and_then(|v| v.as_u64()).map_or(700, |n| n.clamp(200, 8000) as usize);
    let r = crate::refs::map(std::path::Path::new(dir), budget);
    crate::usage::record("map", r.raw_tokens.max(ntok(&r.text)), ntok(&r.text), dir);
    Ok(r.text)
}

fn tool_graph(a: &Value) -> std::result::Result<String, String> {
    let dir = a.get("path").and_then(|v| v.as_str()).unwrap_or(".");
    let action = arg_str(a, "action")?;
    let t = crate::graph::text_for(action, a.get("a").and_then(|v| v.as_str()), a.get("b").and_then(|v| v.as_str()), std::path::Path::new(dir))?;
    crate::usage::record("graph", ntok(&t) * 4, ntok(&t), action);
    Ok(t)
}

fn tool_outline(a: &Value) -> std::result::Result<String, String> {
    let path = arg_str(a, "path")?;
    let (src, lang, _) = read_source(path)?;
    let o = outline(lang.name(), &src).map_err(|e| e.to_string())?;
    let out = format!("# {path} lang={} rev={} {} lines, {} raw tokens\n---\n{o}", lang.name(), fnv(&src), src.lines().count(), ntok(&src));
    crate::usage::record("outline", ntok(&src), ntok(&out), path);
    Ok(out)
}

fn tool_output(a: &Value) -> std::result::Result<String, String> {
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

fn tool_edit(a: &Value) -> std::result::Result<String, String> {
    let path = arg_str(a, "path")?;
    let rev = arg_str(a, "rev")?;
    let (src, lang, crlf) = read_source(path)?;
    if fnv(&src) != rev {
        return Err(format!("{path} changed since it was read (rev {rev} != {}); read it again before editing", fnv(&src)));
    }
    let chunks = a.get("chunks").and_then(|v| v.as_object()).ok_or("missing object argument \"chunks\"")?;
    let mut edits: BTreeMap<String, String> = BTreeMap::new();
    for (k, v) in chunks { edits.insert(k.clone(), v.as_str().ok_or("chunk values must be strings")?.to_string()); }
    let out = apply_chunks(lang.name(), &src, &edits, max_lines(a), false).map_err(|e| e.to_string())?;
    let changed = out != src;
    let delta = (out.lines().count() as i64) - (src.lines().count() as i64);
    if changed && !a.get("dry_run").and_then(|v| v.as_bool()).unwrap_or(false) {
        let text = if crlf { out.replace('\n', "\r\n") } else { out.clone() };
        let tmp = format!("{path}.vtok.tmp");
        std::fs::write(&tmp, &text).map_err(|e| format!("cannot write {tmp}: {e}"))?;
        std::fs::rename(&tmp, path).map_err(|e| format!("cannot replace {path}: {e}"))?;
    }
    Ok(if !changed { format!("{path}: no change (the edited chunks are equivalent to the original)") }
       else { format!("{path}: {} ({:+} lines); new rev={}", if a.get("dry_run").and_then(|v| v.as_bool()).unwrap_or(false) { "dry run, not written" } else { "written" }, delta, fnv(&out)) })
}

fn tool_stats(a: &Value) -> std::result::Result<String, String> {
    let paths = a.get("paths").and_then(|v| v.as_array()).ok_or("missing array argument \"paths\"")?;
    let (mut raw, mut vt) = (0usize, 0usize);
    let mut rows = vec![];
    for p in paths {
        let p = p.as_str().ok_or("paths must be strings")?;
        match read_source(p).and_then(|(src, lang, _)| encode(lang.name(), &src, &Options::default()).map(|r| (src, r)).map_err(|e| e.to_string())) {
            Ok((src, r)) => {
                let (a, b) = (ntok(&src), ntok(&r.vtok) + ntok(&render_table(&r.table)));
                raw += a; vt += b;
                rows.push(format!("{p}: {a} -> {b} ({:.0}%)", 100.0 * (1.0 - b as f64 / a.max(1) as f64)));
            }
            Err(e) => rows.push(format!("{p}: {e}")),
        }
    }
    rows.push(format!("total: {raw} -> {vt} tokens ({:.0}% saved)", 100.0 * (1.0 - vt as f64 / raw.max(1) as f64)));
    Ok(rows.join("\n"))
}

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
