//! `vtok discover`: look through past Claude Code sessions (the local JSONL transcripts) for shell commands whose output
//! was large, and measure what `vtok run` would have shown instead. Read-only; nothing is sent anywhere. The saving is
//! computed by running Vtok's own compactor, with the command's own view, on the output the agent actually received, and
//! only the calls the hook would really wrap count as the hook's saving.
//!
//! `scan` reads the transcripts (each tool call once, however many resumed sessions repeat it), `command_key` names the
//! kind of command, `report` prints the numbers.
pub(crate) mod command_key;
mod report;
mod scan;
#[cfg(test)] mod t_discover;

use std::path::PathBuf;

fn roots() -> Vec<PathBuf> {
    if let Some(d) = std::env::var_os("CLAUDE_CONFIG_DIR") { return vec![PathBuf::from(d).join("projects")]; }
    // Windows shells do not set HOME
    std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(|h| vec![PathBuf::from(h).join(".claude/projects")]).unwrap_or_default()
}

fn jsonl_files(dir: &std::path::Path, since: std::time::SystemTime, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() { jsonl_files(&p, since, out); }
        else if p.extension().map_or(false, |x| x == "jsonl") && e.metadata().and_then(|m| m.modified()).map_or(true, |m| m >= since) { out.push(p); }
    }
}

pub fn run(args: &[String]) -> i32 {
    let value = |flag: &str| args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1));
    let days: u64 = value("--days").and_then(|s| s.parse().ok()).unwrap_or(30);
    let min_tokens: usize = value("--min-tokens").and_then(|s| s.parse().ok()).unwrap_or(400);
    let dirs: Vec<PathBuf> = value("--dir").map(|d| vec![PathBuf::from(d)]).unwrap_or_else(roots);
    let since = std::time::SystemTime::now() - std::time::Duration::from_secs(days * 86400);
    let mut files = vec![];
    for d in &dirs { jsonl_files(d, since, &mut files); }
    if files.is_empty() {
        println!("no Claude Code transcripts found (looked in {}); use --dir DIR", dirs.iter().map(|d| d.display().to_string()).collect::<Vec<_>>().join(", "));
        return 0;
    }
    report::print(&scan::scan(&files, min_tokens), min_tokens);
    0
}
