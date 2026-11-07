//! `vtok refs NAME [DIR]`: where a symbol is defined and where its name is used, across a project.
//!
//! Name-based, not type-resolved: every identifier token equal to NAME counts, grouped by the symbol that contains it.
//! Files that do not contain NAME as text are never parsed, so a query costs one pass of substring checks plus a few parses.
//!
//! `walk` lists the files; `uses` is `vtok refs`, `overview` is `vtok map`, `search` is `vtok find`; the commands'
//! argument parsing and usage records are here. Tests are the `t_*` children.
use std::path::PathBuf;
use vtok_api::ntok;

mod overview;
mod search;
mod uses;
mod walk;
#[cfg(test)] mod t_budget;
#[cfg(test)] mod t_overview;
#[cfg(test)] mod t_search;
#[cfg(test)] mod t_uses;

pub use overview::map;
pub use search::find_symbols;
pub use uses::find;
pub(crate) use walk::{base, walk};

pub struct Report { pub text: String, pub raw_tokens: usize, pub top: Vec<(String, String)> }

pub fn run_map(args: &[String]) -> i32 {
    let pos: Vec<&String> = args.iter().filter(|a| !a.starts_with("--")).collect();
    let dir = pos.iter().find(|a| a.parse::<usize>().is_err()).map(|a| PathBuf::from(a.as_str())).unwrap_or_else(|| PathBuf::from("."));
    let budget = args.iter().position(|a| a == "--max-tokens").and_then(|i| args.get(i + 1)).and_then(|v| v.parse().ok()).unwrap_or(700);
    let r = map(&dir, budget);
    print!("{}", r.text);
    crate::usage::record("map", r.raw_tokens.max(ntok(&r.text)), ntok(&r.text), &dir.display().to_string());
    0
}

pub fn run_find(args: &[String]) -> i32 {
    let budget = args.iter().position(|a| a == "--max-tokens").and_then(|i| args.get(i + 1)).and_then(|v| v.parse().ok()).unwrap_or(400);
    let dir_flag = args.iter().position(|a| a == "--dir").and_then(|i| args.get(i + 1)).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
    let q: Vec<&String> = { let mut skip = false; args.iter().filter(|a| { if skip { skip = false; return false; } if *a == "--max-tokens" || *a == "--dir" { skip = true; return false; } !a.starts_with("--") }).collect() };
    if q.is_empty() { eprintln!("usage: vtok find WORDS... [--dir DIR] [--max-tokens N]"); return 2; }
    let query = q.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(" ");
    let r = find_symbols(&query, &dir_flag, budget);
    print!("{}", r.text);
    crate::usage::record("find", r.raw_tokens.max(ntok(&r.text)), ntok(&r.text), &query);
    0
}

pub fn run(args: &[String]) -> i32 {
    let mut pos = args.iter().filter(|a| !a.starts_with("--"));
    let Some(name) = pos.next() else { eprintln!("usage: vtok refs NAME [DIR] [--max-tokens N]"); return 2 };
    let dir = pos.next().map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
    let budget = args.iter().position(|a| a == "--max-tokens").and_then(|i| args.get(i + 1)).and_then(|v| v.parse().ok()).unwrap_or(500);
    let r = find(name, &dir, budget);
    print!("{}", r.text);
    crate::usage::record("refs", r.raw_tokens.max(ntok(&r.text)), ntok(&r.text), name);
    0
}
