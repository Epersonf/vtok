//! `vtok discover`: look through past Claude Code sessions (the local JSONL transcripts) for shell commands whose output
//! was large, and measure what `vtok run` would have shown instead. Read-only; nothing is sent anywhere. The saving is
//! computed by running Vtok's own compactor on the output the agent actually received.
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use vtok_api::ntok;

fn roots() -> Vec<PathBuf> {
    if let Some(d) = std::env::var_os("CLAUDE_CONFIG_DIR") { return vec![PathBuf::from(d).join("projects")]; }
    std::env::var_os("HOME").map(|h| vec![PathBuf::from(h).join(".claude/projects")]).unwrap_or_default()
}

fn jsonl_files(dir: &std::path::Path, since: std::time::SystemTime, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() { jsonl_files(&p, since, out); }
        else if p.extension().map_or(false, |x| x == "jsonl") && e.metadata().and_then(|m| m.modified()).map_or(true, |m| m >= since) { out.push(p); }
    }
}

fn text_of(c: &Value) -> String {
    match c {
        Value::String(s) => s.clone(),
        Value::Array(a) => a.iter().filter_map(|x| x.get("text").and_then(|t| t.as_str())).collect::<Vec<_>>().join("\n"),
        _ => String::new(),
    }
}

/// Group key: program plus subcommand ("cargo test", "git log", "python -m pytest").
fn key(cmd: &str) -> String {
    let cmd = crate::hook::split_prefix(cmd).1;
    let w: Vec<&str> = cmd.split_whitespace().skip_while(|w| w.contains('=') && !w.starts_with('-') && !w.contains('/')).collect();
    let prog = w.first().map(|p| p.rsplit('/').next().unwrap_or(p)).unwrap_or("?");
    match (prog, w.get(1)) {
        ("python" | "python3", Some(&"-m")) => format!("python -m {}", w.get(2).unwrap_or(&"")),
        (_, Some(s)) if !s.starts_with('-') && !s.contains('/') && !s.contains('.') => format!("{prog} {s}"),
        _ => prog.to_string(),
    }
}

pub fn run(args: &[String]) -> i32 {
    let days: u64 = args.iter().position(|a| a == "--days").and_then(|i| args.get(i + 1)).and_then(|s| s.parse().ok()).unwrap_or(30);
    let min_tokens: usize = args.iter().position(|a| a == "--min-tokens").and_then(|i| args.get(i + 1)).and_then(|s| s.parse().ok()).unwrap_or(400);
    let dirs: Vec<PathBuf> = args.iter().position(|a| a == "--dir").and_then(|i| args.get(i + 1)).map(|d| vec![PathBuf::from(d)]).unwrap_or_else(roots);
    let since = std::time::SystemTime::now() - std::time::Duration::from_secs(days * 86400);
    let mut files = vec![];
    for d in &dirs { jsonl_files(d, since, &mut files); }
    if files.is_empty() { println!("no Claude Code transcripts found (looked in {}); use --dir DIR", dirs.iter().map(|d| d.display().to_string()).collect::<Vec<_>>().join(", ")); return 0; }
    // compaction stores a copy of what it shows; send those to a scratch dir
    let scratch = std::env::temp_dir().join(format!("vtok-discover-{}", std::process::id()));
    std::env::set_var("VTOK_STATE", &scratch);
    std::env::set_var("VTOK_NO_USAGE", "1");
    // group -> (calls, raw tokens, shown tokens, wrapped by the hook?)
    let mut groups: HashMap<String, (u64, u64, u64, bool)> = HashMap::new();
    let (mut calls, mut total_raw) = (0u64, 0u64);
    for f in &files {
        let Ok(text) = std::fs::read_to_string(f) else { continue };
        let mut pending: HashMap<String, String> = HashMap::new();
        for line in text.lines() {
            let Ok(v) = serde_json::from_str::<Value>(line) else { continue };
            let Some(content) = v.pointer("/message/content").and_then(|c| c.as_array()) else { continue };
            for c in content {
                match c.get("type").and_then(|t| t.as_str()) {
                    Some("tool_use") if c.get("name").and_then(|n| n.as_str()) == Some("Bash") => {
                        if let (Some(id), Some(cmd)) = (c.get("id").and_then(|x| x.as_str()), c.pointer("/input/command").and_then(|x| x.as_str())) { pending.insert(id.to_string(), cmd.to_string()); }
                    }
                    Some("tool_result") => {
                        let Some(cmd) = c.get("tool_use_id").and_then(|x| x.as_str()).and_then(|id| pending.remove(id)) else { continue };
                        let out = text_of(c.get("content").unwrap_or(&Value::Null));
                        let raw = ntok(&out) as u64;
                        calls += 1; total_raw += raw;
                        if (raw as usize) < min_tokens { continue; }
                        let code = if out.starts_with("Exit code ") { 1 } else { 0 };
                        let shown = ntok(&crate::shell::compact(&out, code, 700)) as u64;
                        let e = groups.entry(key(&cmd)).or_insert((0, 0, 0, false));
                        e.0 += 1; e.1 += raw; e.2 += shown.min(raw); e.3 |= crate::hook::rewrite(&cmd).is_some();
                    }
                    _ => {}
                }
            }
        }
    }
    let _ = std::fs::remove_dir_all(&scratch);
    let mut rows: Vec<(&String, &(u64, u64, u64, bool))> = groups.iter().collect();
    rows.sort_by_key(|(_, g)| std::cmp::Reverse(g.1 - g.2));
    let (big_raw, big_shown): (u64, u64) = rows.iter().fold((0, 0), |a, (_, g)| (a.0 + g.1, a.1 + g.2));
    println!("{} transcript file(s), {calls} shell calls, {total_raw} tokens of shell output in total", files.len());
    println!("{} command kinds had outputs of {min_tokens}+ tokens: {big_raw} tokens raw, {big_shown} with `vtok run` (saves {}, {:.0}% of those)", rows.len(), big_raw - big_shown, 100.0 * (big_raw - big_shown) as f64 / big_raw.max(1) as f64);
    println!("\n{:<26} {:>5} {:>9} {:>9} {:>7}  hook", "command", "calls", "raw tok", "vtok tok", "saved");
    for (k, g) in rows.iter().take(15) { println!("{:<26} {:>5} {:>9} {:>9} {:>7}  {}", k.chars().take(26).collect::<String>(), g.0, g.1, g.2, g.1 - g.2, if g.3 { "wraps it" } else { "-" }); }
    println!("\nEstimate: Vtok's compactor applied to the output each command really returned. The hook (`vtok init --claude-hook`) rewrites only the commands marked `wraps it`;\nagents that already pipe through head/tail will have smaller raw outputs than a plain run, which this already reflects.");
    0
}
