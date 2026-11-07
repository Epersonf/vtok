//! `vtok run [--max-tokens N] -- COMMAND...` and the MCP `vtok_run` tool.
use super::compact::{build, compact_cmd};
use super::exec::execute;

const DEFAULT_BUDGET: usize = 700;
const USAGE: &str = "usage: vtok run [--max-tokens N] -- COMMAND [ARGS...]";

pub fn run(args: &[String]) -> i32 {
    let mut budget = DEFAULT_BUDGET;
    let mut i = 0;
    while i < args.len() && args[i] != "--" {
        match args[i].as_str() {
            "--max-tokens" => { i += 1; budget = args.get(i).and_then(|s| s.parse().ok()).unwrap_or_else(|| { eprintln!("--max-tokens needs a number"); std::process::exit(2) }); }
            other => { eprintln!("unknown option {other:?}\n{USAGE}"); return 2; }
        }
        i += 1;
    }
    let cmd: Vec<String> = args.iter().skip(i + 1).cloned().collect();
    if cmd.is_empty() { eprintln!("{USAGE}"); return 2; }
    match execute(&cmd, None, None, true) {
        Ok(r) => { print!("{}", compact_cmd(&cmd.join(" "), &String::from_utf8_lossy(&r.out), r.code, budget)); r.code }
        Err(e) => { eprintln!("vtok run: {e}"); 127 }
    }
}

/// `vtok_run` for the MCP server: the same compact view. A failure or timeout is reported first (`[exit N]`); success
/// has no status line, and a clean run that printed nothing says so.
pub fn run_for_mcp(command: &str, cwd: Option<&str>, timeout_s: u64, budget: usize) -> Result<String, String> {
    let r = execute(&[command.to_string()], cwd, Some(std::time::Duration::from_secs(timeout_s.clamp(1, 900))), false)?;
    let c = build(command, &String::from_utf8_lossy(&r.out), r.code, budget, true);
    let mut out = String::new();
    if r.timed_out { out.push_str(&format!("[timed out after {timeout_s}s, killed; exit {}]\n", r.code)); }
    else if r.code != 0 { out.push_str(&format!("[exit {}]\n", r.code)); }
    out.push_str(&c.text);
    out.push_str(&c.footer(r.code, false).unwrap_or_default());
    Ok(if out.is_empty() { "(no output)\n".to_string() } else { out })
}
