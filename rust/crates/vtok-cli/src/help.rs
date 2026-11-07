//! The usage text and the exit helpers every command uses.
pub(crate) const USAGE: &str = "usage: vtok <command> ...
  languages
  encode FILE [--lang L] [--view full|skeleton|lean] [--no-alias] [--hide-literals N] [--table OUT] [--literals OUT]
  decode FILE --lang L [--table T] [--literals L]
  roundtrip PATH... [--lang L] [--no-alias]
  stats PATH... [--lang L] [--no-alias] [--view V]
  split FILE [--lang L] [--max-lines N]          index (skeleton + chunk map) of a big file
  chunk FILE ID [--lang L] [--max-lines N]       full Vtok of one chunk
  apply FILE -e ID=FILE... [-o OUT] [--force] [--lang L] [--max-lines N]
  config [get max-lines | set max-lines N [--local] | unset max-lines [--local]]   persistent split threshold
  run [--max-tokens N] -- COMMAND [ARGS...]      run a command, show a compact view of its output (full output kept)
  output [ID] [--grep A|B] [--lines A-B] [--tail N] [--head N]   read back part of a stored output (no ID: the most recent run)
  find WORDS... [--dir DIR] [--max-tokens N]     symbols whose names match the words of a question\n  map [DIR] [--max-tokens N]                     project overview: most used symbols per file, with signatures\n  refs NAME [DIR] [--max-tokens N]               where a symbol is defined and used (by name, whole project)
  squeeze FILE [--write] | --rewrite FILE | --check A B   shrink a CLAUDE.md / AGENTS.md style file (prose only, code untouched)
  discover [--days N] [--min-tokens N] [--dir D]  find shell commands in past Claude Code sessions that vtok run would have shrunk
  graph build|explain N|path A B|hubs|communities|report|html|hook [--dir D]   persistent relation graph (calls, types, implements, ...) with scores and communities
  rewrite COMMAND...                             print the vtok run form of a long-output command (exit 1 when unchanged), for any agent hook
  filter [--explain] [--code N] -- COMMAND...    show what vtok run would print for output read from stdin (testing filters)
  browse URL [--max-tokens N] [--no-links] [--allow-private]   web page as compact text (full text kept under an id)
  route REQUEST [--files N] [--json]             pick a model tier and effort for a request (small / medium / large)
  proxy [--port N] [--upstream URL]              local API proxy: model auto is routed to the model route picks
  shims install [DIR] | list                     wrapper scripts for cargo/npm/pytest/tsc/... on PATH: compact output for ANY agent, no hook
  init [--hook AGENT [--global] | --agent NAME | --write FILE | --claude-hook SETTINGS] [--terse]   usage note for any agent (AGENTS.md etc.), as a fenced block; --terse adds the answer-style note (alone: prints it)
  gain [--days N | --since 2h] [--session current|ID] [--cwd DIR] [--legacy] [--json | --line]   tokens saved by Vtok's tools (local usage log)
  session start NAME | stop | status | list [--all] | report [NAME] [--whole] [--per-call] [--by-prompt] [--price-in USD] [--json]   name a stretch of work; its report sets the tokens saved against what the turns cost (Claude Code transcript)
  statusline                                     one line for a status bar: what vtok saved in this chat (reads the agent's status JSON on stdin); init --claude-statusline SETTINGS registers it
  outline FILE [--lang L]                        names and line ranges of a file (cheapest overview)
  symbols FILE [--lang L]                        id and line range of every addressable symbol
  grammar --lang L
  mcp [--allow-run]                              MCP server on stdio (vtok_read/chunk/edit/outline/stats/output/gain; vtok_run only with --allow-run)";

pub(crate) fn die(msg: &str, code: i32) -> ! { eprintln!("{msg}"); std::process::exit(code) }
pub(crate) fn usage() -> ! { die(USAGE, 2) }
