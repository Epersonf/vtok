//! Persistent settings (`vtok config`). Today: `max-lines`, the size above which a file is split into chunks.
//!
//! Resolution order, first hit wins: `--max-lines` / `max_lines` argument, `VTOK_MAX_LINES`, the nearest
//! `.vtok.json` walking up from the file, the user file (`$VTOK_CONFIG`, else `$XDG_CONFIG_HOME/vtok/config.json`,
//! else `~/.config/vtok/config.json`), then 150.
//!
//! `lookup` reads the settings (and says whether the MCP server may run commands), `store` writes them; `run` here is
//! the `vtok config` command.
mod lookup;
mod store;
#[cfg(test)] mod t_config;

pub use lookup::{local_config_path, max_lines, mcp_run_enabled, user_config_path, ALLOW_RUN};
pub use store::{set_max_lines, set_mcp_run, unset_max_lines};

/// `vtok config [get|set|unset] ...` (args after the word `config`). Returns the exit code.
pub fn run(args: &[String]) -> i32 {
    let local = args.iter().any(|a| a == "--local");
    let words: Vec<&String> = args.iter().filter(|a| !a.starts_with("--")).collect();
    let w = |i: usize| words.get(i).map(|s| s.as_str());
    let key_ok = |k: Option<&str>| matches!(k, Some("max-lines") | Some("max_lines"));
    match w(0) {
        None | Some("show") => {
            let (n, src) = max_lines(None);
            println!("max-lines = {n}  ({src})");
            println!("mcp-run = {}  (lets the MCP server run commands via vtok_run; user-level only)", if mcp_run_enabled() { "on" } else { "off" });
            println!("user config: {}", user_config_path().map_or("-".into(), |p| p.display().to_string()));
            println!("repo config: {}", std::env::current_dir().ok().and_then(|d| local_config_path(&d)).map_or("-".into(), |p| p.display().to_string()));
            0
        }
        Some("set") if matches!(w(1), Some("mcp-run")) => {
            if local { eprintln!("error: mcp-run can only be set for the user, not with --local"); return 2; }
            match w(2) {
                Some("on") | Some("off") => match set_mcp_run(w(2) == Some("on")) {
                    Ok(p) => { println!("mcp-run = {}  ({})", w(2).unwrap_or("off"), p.display()); 0 }
                    Err(e) => { eprintln!("error: {e}"); 2 }
                },
                _ => { eprintln!("usage: vtok config set mcp-run on|off"); 2 }
            }
        }
        Some("get") if key_ok(w(1)) => { println!("{}", max_lines(None).0); 0 }
        Some("set") if key_ok(w(1)) => match w(2).and_then(|s| s.parse::<usize>().ok()) {
            Some(n) => match set_max_lines(n, local) {
                Ok(p) => { println!("max-lines = {n}  ({})", p.display()); 0 }
                Err(e) => { eprintln!("error: {e}"); 2 }
            },
            None => { eprintln!("usage: vtok config set max-lines N [--local]"); 2 }
        },
        Some("unset") if key_ok(w(1)) => match unset_max_lines(local) {
            Ok(p) => { println!("max-lines unset in {}", p.display()); 0 }
            Err(e) => { eprintln!("error: {e}"); 2 }
        },
        _ => {
            eprintln!("usage: vtok config [get max-lines | set max-lines N [--local] | unset max-lines [--local] | set mcp-run on|off]");
            2
        }
    }
}
