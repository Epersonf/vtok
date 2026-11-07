//! What the server announces: the instructions text and the tool list.
use serde_json::{json, Value};

/// What the server tells every session that has it registered, on every turn: short on purpose. The notation rules of a
/// language are not here; they come with the first code shown in it (see `notation`).
pub(super) fn instructions() -> String {
    concat!(
        "Vtok returns source code in a compact notation (comments dropped, names aliased, whitespace minimal), and the plain text when that is cheaper, ",
        "so use its tools for any Python / TypeScript / JavaScript / Rust / Java / Go / C / C++ / C# / Dart / CSS / HTML / SQL file instead of reading it whole. ",
        "vtok_outline lists the name and line range of every symbol (the cheapest overview); vtok_find locates code from the words of a question ",
        "(show=1 adds the best match's code); vtok_refs shows where a name is defined and used; vtok_chunk returns one symbol (`name`, or `Class.method`); ",
        "vtok_read reads a file (a big one comes back as an index plus chunk ids like c3). The first code you receive in a language is preceded by that notation's rules.\n\n",
        "To change code, send vtok_edit the edited notation of a chunk or symbol with the rev from the read (the server splices it into the original file and refuses if the file changed since). ",
        "For a small tweak, or a file with no notation (Markdown, TOML, JSON...), send replace=[{old, new}]: exact text, checked against the rev. ",
        "The read tools return plain text, with its rev, when the compact form would save less than the min-gain setting, so there is nothing to decide beforehand.\n\n",
        "Long command output: run `vtok run -- <command>` in the shell (or vtok_run, if it is listed). You get a compact view; ",
        "when something was cut or the command failed, the last line names an id and vtok_output(grep/lines/tail) returns any part of the full output.\n",
    )
    .to_string()
}

pub(super) fn tools() -> Value {
    let all = json!([
      {"name": "vtok_read",
       "description": "Compact Vtok of a Python/TypeScript/JavaScript/Rust/Java/Go/C/C++/C#/Dart/CSS/HTML/SQL file: rev, alias table, code. Big files return an index + chunk map. view: full, skeleton or lean. When Vtok would save less than min-gain, or the file has no Vtok notation, the plain text comes back instead, with the same rev.",
       "inputSchema": {"type": "object", "properties": {
          "path": {"type": "string"}, "view": {"type": "string", "enum": ["full", "skeleton", "lean"]},
          "max_lines": {"type": "integer", "description": "split files longer than this (default: `vtok config` max-lines, else 150)"}}, "required": ["path"]}},
      {"name": "vtok_chunk",
       "description": "Compact code of one symbol (name, or Class.method) or chunk id (c3), with its rev and aliases (the plain lines when that is smaller).",
       "inputSchema": {"type": "object", "properties": {"path": {"type": "string"}, "id": {"type": "string"}, "max_lines": {"type": "integer"}}, "required": ["path", "id"]}},
      {"name": "vtok_edit",
       "description": "Edit a file, checked against the rev from the read/chunk. Either chunks (id -> complete new Vtok; untouched tokens keep their original text) or replace (a list of {old, new, all?}: exact text, each old must occur once unless all is true; works on any text file). dry_run reports without writing.",
       "inputSchema": {"type": "object", "properties": {
          "path": {"type": "string"}, "rev": {"type": "string"},
          "chunks": {"type": "object", "additionalProperties": {"type": "string"}},
          "replace": {"type": "array", "items": {"type": "object", "properties": {"old": {"type": "string"}, "new": {"type": "string"}, "all": {"type": "boolean"}}, "required": ["old", "new"]}},
          "max_lines": {"type": "integer"}, "dry_run": {"type": "boolean"}}, "required": ["path", "rev"]}},
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
       "description": "Tokens saved by Vtok's tools on this machine (local usage log): calls, raw vs shown tokens per tool. Only run (exact) and read/chunk/outline (upper bound: the whole file) are counted; session=current limits it to this chat.",
       "inputSchema": {"type": "object", "properties": {"days": {"type": "integer", "description": "default 30"}, "session": {"type": "string", "description": "\"current\" or a chat session id"}, "since": {"type": "string", "description": "90s, 30m, 2h, 7d"}, "legacy": {"type": "boolean", "description": "include records from before session tags"}}}},
      {"name": "vtok_session",
       "description": "Name a stretch of work and see what Vtok saved in it. action: start (name) | stop (prints the report) | status | list | report (newest stretch; whole: the whole chat; by_prompt: per prompt). Tokens not sent, set against the chat's cost; only an A/B run shows the real effect.",
       "inputSchema": {"type": "object", "properties": {"action": {"type": "string"}, "name": {"type": "string"}, "whole": {"type": "boolean"}, "by_prompt": {"type": "boolean"}}}},
      {"name": "vtok_stats",
       "description": "Token counts of files: raw vs Vtok (incl. alias table). Use it to decide whether vtok_read pays off.",
       "inputSchema": {"type": "object", "properties": {"paths": {"type": "array", "items": {"type": "string"}}}, "required": ["paths"]}}
    ]);
    let mut v = all;
    if let Some(a) = v.as_array_mut() { if !crate::config::mcp_run_enabled() { a.retain(|t| t["name"] != "vtok_run"); }
        // fewer tool schemas = fewer tokens on every turn; these stay available from the command line (and as MCP calls)
        if std::env::var("VTOK_MCP_ALL").map_or(true, |v| v != "1") { a.retain(|t| !matches!(t["name"].as_str(), Some("vtok_gain") | Some("vtok_session") | Some("vtok_stats") | Some("vtok_map") | Some("vtok_graph") | Some("vtok_browse"))); } }
    v
}
