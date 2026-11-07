//! Persistent settings (`vtok config`): `max-lines` (the size above which a file is split into chunks), `min-gain` (how much
//! the compact form must save before `vtok_read` / `vtok_chunk` use it instead of the plain text), `run-tiny` (the output
//! size `vtok run` prints unchanged) and `mcp-run` (whether the MCP server may run commands).
//!
//! The numbers resolve in this order, first hit wins: the environment variable (`VTOK_MAX_LINES`, `VTOK_MIN_GAIN`,
//! `VTOK_RUN_TINY`), the nearest `.vtok.json` walking up from the file, the user file (`$VTOK_CONFIG`, else
//! `$XDG_CONFIG_HOME/vtok/config.json`, else `~/.config/vtok/config.json`), then the default. A value outside its range is
//! ignored. (`vtok split` and `vtok chunk` also take `--max-lines`.)
//!
//! `settings` describes the numbers, `lookup` reads the files (and says whether the MCP server may run commands), `store`
//! writes them; `run` here is the `vtok config` command.
mod lookup;
mod settings;
mod store;
#[cfg(test)] mod t_config;
#[cfg(test)] mod t_settings;

pub use lookup::{local_config_path, max_lines, mcp_run_enabled, user_config_path, ALLOW_RUN};
pub use store::set_mcp_run;
use settings::{find, Setting, ALL};
use std::path::{Path, PathBuf};
use store::{set_number, unset_number};

/// Percent of tokens the compact form must save before `vtok_read` / `vtok_chunk` return it instead of the plain text.
pub fn min_gain(near: Option<&Path>) -> usize {
    settings::MIN_GAIN.get(near).0
}

/// `vtok run` prints an output of up to this many tokens unchanged (read once per process).
pub fn run_tiny() -> usize {
    static N: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
    *N.get_or_init(|| settings::RUN_TINY.get(None).0)
}

/// `vtok config [get|set|unset] ...` (args after the word `config`). Returns the exit code.
pub fn run(args: &[String]) -> i32 {
    let local = args.iter().any(|a| a == "--local");
    let words: Vec<&String> = args.iter().filter(|a| !a.starts_with("--")).collect();
    let w = |i: usize| words.get(i).map(|s| s.as_str());
    match (w(0), w(1)) {
        (None, _) | (Some("show"), _) => show(),
        (Some("set"), Some("mcp-run")) => set_mcp(w(2), local),
        (Some(verb @ ("get" | "set" | "unset")), Some(name)) => match find(name) {
            Some(s) => number(verb, s, w(2), local),
            None => usage(),
        },
        _ => usage(),
    }
}

fn show() -> i32 {
    for s in ALL.iter() {
        let (n, src) = s.get(None);
        println!("{} = {n}  ({src})  {}", s.name, s.about);
    }
    println!("mcp-run = {}  (lets the MCP server run commands via vtok_run; user-level only)", if mcp_run_enabled() { "on" } else { "off" });
    println!("user config: {}", user_config_path().map_or("-".into(), |p| p.display().to_string()));
    println!("repo config: {}", std::env::current_dir().ok().and_then(|d| local_config_path(&d)).map_or("-".into(), |p| p.display().to_string()));
    0
}

fn set_mcp(value: Option<&str>, local: bool) -> i32 {
    if local {
        eprintln!("error: mcp-run can only be set for the user, not with --local");
        return 2;
    }
    match value {
        Some(v @ ("on" | "off")) => done(set_mcp_run(v == "on"), |p| format!("mcp-run = {v}  ({})", p.display())),
        _ => {
            eprintln!("usage: vtok config set mcp-run on|off");
            2
        }
    }
}

fn number(verb: &str, s: &Setting, value: Option<&str>, local: bool) -> i32 {
    match verb {
        "get" => {
            println!("{}", s.get(None).0);
            0
        }
        "set" => match value.and_then(|v| v.parse::<usize>().ok()) {
            Some(n) => done(set_number(s, n, local), |p| format!("{} = {n}  ({})", s.name, p.display())),
            None => {
                eprintln!("usage: vtok config set {} N [--local]   (N from {} to {})", s.name, s.min, s.max);
                2
            }
        },
        _ => done(unset_number(s, local), |p| format!("{} unset in {}", s.name, p.display())),
    }
}

fn done(result: Result<PathBuf, String>, message: impl Fn(&PathBuf) -> String) -> i32 {
    match result {
        Ok(p) => {
            println!("{}", message(&p));
            0
        }
        Err(e) => {
            eprintln!("error: {e}");
            2
        }
    }
}

fn usage() -> i32 {
    let names: Vec<&str> = ALL.iter().map(|s| s.name).collect();
    eprintln!("usage: vtok config [show | get NAME | set NAME N [--local] | unset NAME [--local] | set mcp-run on|off]   NAME: {}", names.join(", "));
    2
}
