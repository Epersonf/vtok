//! The Claude Code transcript of a chat (`~/.claude/projects/<project>/<session id>.jsonl`): the token usage of every
//! turn, the user's prompts and the compactions. Read only; lines that are not what is looked for are never parsed, so
//! a 35 MB transcript loads in a fraction of a second. Subagents have transcripts of their own and are not in this one.
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Usage { pub input: u64, pub cache_read: u64, pub cache_write: u64, pub output: u64 }

/// One assistant message: when it started and what it used.
#[derive(Clone, Debug)]
pub struct Turn { pub t: f64, pub usage: Usage }

/// One thing the user typed.
#[derive(Clone, Debug)]
pub struct Prompt { pub t: f64 }

#[derive(Clone, Debug, Default)]
pub struct Transcript { pub turns: Vec<Turn>, pub prompts: Vec<Prompt>, pub compactions: Vec<f64>, pub model: Option<String> }

/// Seconds since 1970 of `2026-11-07T03:48:45.199Z`.
pub fn iso_secs(s: &str) -> Option<f64> {
    let (date, time) = s.trim_end_matches('Z').split_once('T')?;
    let mut d = date.split('-').map(|x| x.parse::<i64>().ok());
    let (y, m, day) = (d.next()??, d.next()??, d.next()??);
    let mut t = time.split(':');
    let (h, mi) = (t.next()?.parse::<i64>().ok()?, t.next()?.parse::<i64>().ok()?);
    let sec: f64 = t.next()?.parse().ok()?;
    Some(days_from_civil(y, m, day) as f64 * 86_400.0 + (h * 3600 + mi * 60) as f64 + sec)
}

/// Days from 1970-01-01 to a calendar date (Howard Hinnant's algorithm).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = (if y >= 0 { y } else { y - 399 }) / 400;
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    era * 146_097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719_468
}

/// Where Claude Code keeps its projects: `$CLAUDE_CONFIG_DIR/projects`, else `~/.claude/projects`.
fn project_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(c) = std::env::var_os("CLAUDE_CONFIG_DIR") { roots.push(PathBuf::from(c).join("projects")); }
    if let Some(h) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) { roots.push(PathBuf::from(h).join(".claude/projects")); }
    roots
}

/// The transcript file of a chat, found by its session id (an id with anything but letters, digits, `-` and `_` is refused).
pub fn find(session: &str) -> Option<PathBuf> {
    if session.is_empty() || !session.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_') { return None; }
    for root in project_roots() {
        let Ok(dirs) = std::fs::read_dir(&root) else { continue };
        if let Some(p) = dirs.flatten().map(|e| e.path().join(format!("{session}.jsonl"))).find(|p| p.is_file()) { return Some(p); }
    }
    None
}

pub fn load(path: &std::path::Path) -> Transcript { parse(&String::from_utf8_lossy(&std::fs::read(path).unwrap_or_default())) }

/// Typed by the user: a plain string, or blocks with no tool result (a tool result also arrives as a user line).
fn typed_by_user(content: &Value) -> bool {
    match content {
        Value::String(_) => true,
        Value::Array(blocks) => !blocks.iter().any(|b| b["type"] == "tool_result"),
        _ => false,
    }
}

pub fn parse(text: &str) -> Transcript {
    let mut by_id: HashMap<String, Turn> = HashMap::new();
    let mut tr = Transcript::default();
    for line in text.lines() {
        let assistant = line.contains("\"type\":\"assistant\"") && line.contains("\"usage\"");
        let compaction = line.contains("\"isCompactSummary\":true");
        let prompt = line.contains("\"type\":\"user\"") && line.contains("\"kind\":\"human\"");
        if !(assistant || compaction || prompt) { continue; }
        let Ok(v) = serde_json::from_str::<Value>(line) else { continue };
        let Some(t) = v["timestamp"].as_str().and_then(iso_secs) else { continue };
        if v["isSidechain"] == true { continue; }
        if compaction { tr.compactions.push(t); continue; }
        if assistant {
            let m = &v["message"];
            let Some(id) = m["id"].as_str() else { continue };
            let u = &m["usage"];
            let n = |k: &str| u[k].as_u64().unwrap_or(0);
            let now = Usage { input: n("input_tokens"), cache_read: n("cache_read_input_tokens"), cache_write: n("cache_creation_input_tokens"), output: n("output_tokens") };
            if tr.model.is_none() { tr.model = m["model"].as_str().map(String::from); }
            // one message arrives as several lines (thinking, text, tool use): keep the first time and the largest counts
            let e = by_id.entry(id.to_string()).or_insert(Turn { t, usage: now });
            e.t = e.t.min(t);
            e.usage = Usage { input: e.usage.input.max(now.input), cache_read: e.usage.cache_read.max(now.cache_read), cache_write: e.usage.cache_write.max(now.cache_write), output: e.usage.output.max(now.output) };
        } else if typed_by_user(&v["message"]["content"]) {
            tr.prompts.push(Prompt { t });
        }
    }
    tr.turns = by_id.into_values().collect();
    tr.turns.sort_by(|a, b| a.t.total_cmp(&b.t));
    tr.prompts.sort_by(|a, b| a.t.total_cmp(&b.t));
    tr.compactions.sort_by(|a, b| a.total_cmp(b));
    tr
}
