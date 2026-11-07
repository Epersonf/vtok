use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use vtok_api::*;
mod agents;
mod auto;
mod browse;
mod config;
mod data;
mod diag;
mod discover;
mod filter_cmd;
mod filters;
mod filters_tools;
mod init;
mod gitfmt;
mod gitdiff;
mod graph;
mod graph_out;
mod hook;
mod html_text;
mod hook_agents;
mod hook_install;
mod hook_plugins;
mod risk;
mod mcp;
mod proxy;
mod refs;
mod route;
mod rewrite;
mod shell;
mod squeeze;
mod terse;
#[cfg(test)] mod t_retention;
mod tomlfilters;
mod usage;
mod cmdviews;
mod views_build;
mod views_lang;
mod views_sys;
mod userfilters;

const USAGE: &str = "usage: vtok <command> ...
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
  gain [--days N] [--json]                       tokens saved by Vtok's tools (local usage log)
  outline FILE [--lang L]                        names and line ranges of a file (cheapest overview)
  symbols FILE [--lang L]                        id and line range of every addressable symbol
  grammar --lang L
  mcp [--allow-run]                              MCP server on stdio (vtok_read/chunk/edit/outline/stats/output/gain; vtok_run only with --allow-run)";

fn die(msg: &str, code: i32) -> ! { eprintln!("{msg}"); std::process::exit(code) }
fn usage() -> ! { die(USAGE, 2) }

const VALUE_FLAGS: [&str; 8] = ["--view", "--hide-literals", "--table", "--literals", "--lang", "--max-lines", "-e", "-o"];

fn flag(args: &[String], name: &str) -> Option<String> {
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1).cloned())
}

fn flags(args: &[String], name: &str) -> Vec<String> {
    args.iter().enumerate().filter(|(_, a)| *a == name).filter_map(|(i, _)| args.get(i + 1).cloned()).collect()
}

fn has(args: &[String], name: &str) -> bool { args.iter().any(|a| a == name) }

fn positional(args: &[String]) -> Vec<String> {
    let mut out = vec![];
    let mut skip = false;
    for a in args {
        if skip { skip = false; continue; }
        if VALUE_FLAGS.contains(&a.as_str()) { skip = true; continue; }
        if a.starts_with('-') && a.len() > 1 { continue; }
        out.push(a.clone());
    }
    out
}

fn lang_for(args: &[String], path: &str) -> &'static dyn Language {
    match flag(args, "--lang") {
        Some(l) => language(&l).unwrap_or_else(|e| die(&format!("error: {e}"), 2)),
        None => language_for_path(path).unwrap_or_else(|e| die(&format!("error: {e}"), 2)),
    }
}

fn options(args: &[String]) -> Options {
    Options {
        alias: Some(!has(args, "--no-alias")),
        view: flag(args, "--view"),
        hide_literals: flag(args, "--hide-literals").map(|n| n.parse().unwrap_or_else(|_| usage())),
        ..Default::default()
    }
}

fn read(path: &str) -> String { std::fs::read_to_string(path).unwrap_or_else(|e| die(&format!("error: cannot read {path}: {e}"), 2)) }

fn collect(paths: &[String], lang: Option<&str>, out: &mut Vec<PathBuf>) {
    for p in paths {
        let p = Path::new(p);
        if p.is_dir() {
            let mut kids: Vec<PathBuf> = std::fs::read_dir(p).unwrap().filter_map(|e| e.ok()).map(|e| e.path()).collect();
            kids.sort();
            collect(&kids.iter().map(|k| k.to_string_lossy().into_owned()).collect::<Vec<_>>(), lang, out);
        } else {
            let s = p.to_string_lossy();
            let ok = match lang { Some(l) => language(l).map_or(false, |l| l.extensions().iter().any(|e| s.ends_with(e))), None => language_for_path(&s).is_ok() };
            if ok && !s.ends_with(".d.ts") { out.push(p.to_path_buf()); }
        }
    }
}

/// Run the CLI with `args` (without the program name); returns the exit code.
pub fn run(args: &[String]) -> i32 {
    let Some(cmd) = args.first() else { usage() };
    let rest = &args[1..];
    let code = match cmd.as_str() {
        "mcp" => mcp::serve(rest),
        "languages" => {
            for l in languages() { println!("{:12} {}", l.name(), l.extensions().join(" ")); }
            0
        }
        "grammar" => {
            let l = language(&flag(rest, "--lang").unwrap_or_else(|| usage())).unwrap_or_else(|e| die(&format!("error: {e}"), 2));
            println!("{}", l.grammar_prompt());
            0
        }
        "encode" => {
            let pos = positional(rest);
            let file = pos.first().unwrap_or_else(|| usage());
            let l = lang_for(rest, file);
            let o = options(rest).to_opts().unwrap_or_else(|e| die(&format!("error: {e}"), 2));
            match l.encode(&read(file), &o) {
                Ok(r) => {
                    print!("{}", r.vtok);
                    if !r.table.is_empty() {
                        eprintln!("# alias table\n{}", render_table(&r.table));
                        if let Some(t) = flag(rest, "--table") { std::fs::write(t, render_table(&r.table) + "\n").unwrap(); }
                    }
                    if let (false, Some(p)) = (r.literals.is_empty(), flag(rest, "--literals")) { std::fs::write(p, json_map(&r.literals)).unwrap(); }
                    0
                }
                Err(e) => { eprintln!("error: {e}"); 1 }
            }
        }
        "decode" => {
            let pos = positional(rest);
            let file = pos.first().unwrap_or_else(|| usage());
            let l = lang_for(rest, file);
            let table = flag(rest, "--table").map(|t| parse_table(&read(&t))).unwrap_or_default();
            let lits: HashMap<String, String> = flag(rest, "--literals").map(|p| parse_json_map(&read(&p))).unwrap_or_default();
            match l.decode(&read(file), &table, &lits) {
                Ok(s) => { print!("{s}"); 0 }
                Err(e) => { eprintln!("parse error: {e}"); 1 }
            }
        }
        "roundtrip" | "stats" => {
            let mut files = vec![];
            let lang_name = flag(rest, "--lang");
            collect(&positional(rest), lang_name.as_deref(), &mut files);
            let o = options(rest).to_opts().unwrap_or_else(|e| die(&format!("error: {e}"), 2));
            let (mut ok, mut fail, mut raw, mut vt) = (0, 0, 0usize, 0usize);
            for f in &files {
                let path = f.to_string_lossy().into_owned();
                let src = std::fs::read_to_string(f).unwrap_or_default();
                match lang_for(rest, &path).encode(&src, &o) {
                    Ok(r) => {
                        ok += 1;
                        if cmd == "stats" { raw += ntok(&src); vt += ntok(&r.vtok) + ntok(&render_table(&r.table)); }
                    }
                    Err(e) => { fail += 1; eprintln!("FAIL {path}: {e}"); }
                }
            }
            println!("roundtrip ok={ok} fail={fail}");
            if cmd == "stats" { println!("tokens raw={raw} vtok+table={vt} saving={:.1}%", 100.0 * (1.0 - vt as f64 / raw.max(1) as f64)); }
            if fail > 0 { 1 } else { 0 }
        }
        "config" => config::run(rest),
        "run" => shell::run(rest),
        "gain" => usage::gain(rest),
        "hook" => hook::run(rest),
        "rewrite" => rewrite::run(rest),
        "browse" => browse::run(rest),
        "filter" => filter_cmd::run(rest),
        "route" => route::run(rest),
        "proxy" => proxy::run(rest),
        "auto" => auto::run(rest),
        "shims" => auto::shims_cmd(rest),
        "squeeze" => squeeze::run(rest),
        "discover" => discover::run(rest),
        "graph" => graph::run(rest),
        "init" => init::run(rest),
        "refs" => refs::run(rest),
        "map" => refs::run_map(rest),
        "find" => refs::run_find(rest),
        "output" if rest.iter().any(|a| a == "--file") => gitdiff::output_file(rest),
        "output" => shell::output_cmd(rest),
        "outline" => {
            let pos = positional(rest);
            let file = pos.first().unwrap_or_else(|| usage());
            let l = lang_for(rest, file);
            match outline(l.name(), &read(file)) {
                Ok(s) => { print!("{s}"); 0 }
                Err(e) => { eprintln!("error: {e}"); 1 }
            }
        }
        "symbols" => {
            let pos = positional(rest);
            let file = pos.first().unwrap_or_else(|| usage());
            let l = lang_for(rest, file);
            match symbols(l.name(), &read(file)) {
                Ok(v) => { for (id, a, b) in v { println!("{id}\t{a}\t{b}"); } 0 }
                Err(e) => { eprintln!("error: {e}"); 1 }
            }
        }
        "split" | "chunk" | "apply" => {
            let pos = positional(rest);
            let file = pos.first().unwrap_or_else(|| usage());
            let l = lang_for(rest, file);
            let max_lines: usize = flag(rest, "--max-lines").map(|n| n.parse().unwrap_or_else(|_| usage())).unwrap_or_else(|| config::max_lines(Some(Path::new(file))).0);
            let src = read(file);
            let res = (|| -> Result<i32> {
                match cmd.as_str() {
                    "split" => {
                        let s = split_file(l.name(), &src, max_lines)?;
                        let raw = ntok(&normalize_newlines(&src));
                        print!("{}", s.index);
                        if s.whole_file {
                            eprintln!("# {} lines <= {max_lines}: not split (c1 = whole file)", src.lines().count());
                        } else {
                            if !s.table.is_empty() { eprintln!("# alias table\n{}", render_table(&s.table)); }
                            let n = ntok(&s.index);
                            eprintln!("# raw={raw} tokens; index={n} ({}%); {} chunks", 100 * n / raw.max(1), s.chunks.len());
                        }
                    }
                    "chunk" => {
                        let id = pos.get(1).unwrap_or_else(|| usage());
                        print!("{}", chunk_of(l.name(), &src, id, max_lines)?);
                    }
                    _ => {
                        let mut edits: BTreeMap<String, String> = BTreeMap::new();
                        for spec in flags(rest, "-e") {
                            let (id, path) = spec.split_once('=').unwrap_or_else(|| usage());
                            edits.insert(id.to_string(), read(path));
                        }
                        let out = apply_chunks(l.name(), &src, &edits, max_lines, has(rest, "--force"))?;
                        match flag(rest, "-o") { Some(o) => std::fs::write(o, out).unwrap(), None => print!("{out}") }
                    }
                }
                Ok(0)
            })();
            match res { Ok(c) => c, Err(e) => { eprintln!("error: {e}"); 1 } }
        }
        _ => usage(),
    };
    code
}

// minimal JSON (string -> string) so the CLI needs no extra dependency
fn json_map(m: &BTreeMap<String, String>) -> String {
    let esc = |s: &str| s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n").replace('\r', "\\r").replace('\t', "\\t");
    format!("{{{}}}", m.iter().map(|(k, v)| format!("\"{}\":\"{}\"", esc(k), esc(v))).collect::<Vec<_>>().join(","))
}

fn parse_json_map(s: &str) -> HashMap<String, String> {
    let b: Vec<char> = s.chars().collect();
    let mut i = 0;
    let mut out = HashMap::new();
    let read_str = |i: &mut usize| -> Option<String> {
        while *i < b.len() && b[*i] != '"' { if b[*i] == '}' { return None; } *i += 1; }
        if *i >= b.len() { return None; }
        *i += 1;
        let mut r = String::new();
        while *i < b.len() && b[*i] != '"' {
            if b[*i] == '\\' {
                *i += 1;
                match b[*i] {
                    'n' => r.push('\n'), 'r' => r.push('\r'), 't' => r.push('\t'),
                    'u' => { let h: String = b[*i + 1..*i + 5].iter().collect(); r.push(char::from_u32(u32::from_str_radix(&h, 16).unwrap_or(63)).unwrap_or('?')); *i += 4; }
                    c => r.push(c),
                }
            } else { r.push(b[*i]); }
            *i += 1;
        }
        *i += 1;
        Some(r)
    };
    while let Some(k) = read_str(&mut i) {
        match read_str(&mut i) { Some(v) => { out.insert(k, v); } None => break }
    }
    out
}
