//! Reading the transcripts: every large shell output and every large Read result, each tool call counted once even when a
//! resumed session repeats it in another file.
use super::command_key::key;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use vtok_api::{language_for_path, ntok};

const PROSE: [&str; 6] = ["txt", "md", "rst", "adoc", "tex", "org"];
const DATA: [&str; 10] = ["json", "jsonl", "ndjson", "csv", "tsv", "yaml", "yml", "toml", "xml", "log"];

/// A shell call whose output was large: its group, what it returned, what `vtok run` shows instead, and whether the hook
/// (`vtok init --claude-hook`) would have wrapped the command.
pub struct Call {
    pub key: String,
    pub raw: u64,
    pub shown: u64,
    pub hooked: bool,
}

/// Tokens of large Read results by kind of file.
#[derive(Default)]
pub struct Reads {
    pub calls: u64,
    pub prose: u64,
    pub code: u64,
    pub data: u64,
    pub other: u64,
}

#[derive(Default)]
pub struct Scan {
    pub files: usize,
    pub shell_calls: u64,
    pub shell_raw: u64,
    pub big: Vec<Call>,
    pub reads: Reads,
}

/// What a tool call asked for, kept until its result shows up.
enum Pending {
    Shell { cmd: String, cwd: Option<String> },
    Read { path: String },
}

fn text_of(c: &Value) -> String {
    match c {
        Value::String(s) => s.clone(),
        Value::Array(a) => a.iter().filter_map(|x| x.get("text").and_then(|t| t.as_str())).collect::<Vec<_>>().join("\n"),
        _ => String::new(),
    }
}

fn pending_of(call: &Value, cwd: Option<&str>) -> Option<Pending> {
    let input = call.get("input")?;
    match call.get("name").and_then(|n| n.as_str())? {
        "Bash" => Some(Pending::Shell { cmd: input.get("command")?.as_str()?.to_string(), cwd: cwd.map(String::from) }),
        "Read" => Some(Pending::Read { path: input.get("file_path")?.as_str()?.to_string() }),
        _ => None,
    }
}

fn add_read(r: &mut Reads, path: &str, tokens: u64) {
    let ext = path.rsplit_once('.').map(|x| x.1.to_ascii_lowercase()).unwrap_or_default();
    r.calls += 1;
    if PROSE.contains(&ext.as_str()) { r.prose += tokens }
    else if DATA.contains(&ext.as_str()) { r.data += tokens }
    else if language_for_path(path).is_ok() { r.code += tokens }
    else { r.other += tokens }
}

fn record(s: &mut Scan, p: Pending, out: &str, min_tokens: usize) {
    let raw = ntok(out) as u64;
    match p {
        Pending::Shell { cmd, cwd } => {
            s.shell_calls += 1;
            s.shell_raw += raw;
            if (raw as usize) < min_tokens { return; }
            let code = if out.starts_with("Exit code ") { 1 } else { 0 };
            // the view the command would get from `vtok run`, nothing stored
            let shown = (ntok(&crate::shell::preview(&cmd, out, code, 700)) as u64).min(raw);
            let hooked = crate::rewrite::rewrite_in(&cmd, false, cwd.as_deref()).is_some();
            s.big.push(Call { key: key(&cmd), raw, shown, hooked });
        }
        Pending::Read { path } => {
            if (raw as usize) >= min_tokens { add_read(&mut s.reads, &path, raw); }
        }
    }
}

pub fn scan(files: &[PathBuf], min_tokens: usize) -> Scan {
    let mut s = Scan { files: files.len(), ..Default::default() };
    let mut seen: HashSet<String> = HashSet::new();
    for f in files {
        let Ok(file) = std::fs::File::open(f) else { continue };
        let mut pending: HashMap<String, Pending> = HashMap::new();
        for line in BufReader::new(file).split(b'\n').flatten() {
            let Ok(v) = serde_json::from_slice::<Value>(&line) else { continue };
            let Some(content) = v.pointer("/message/content").and_then(|c| c.as_array()) else { continue };
            let cwd = v.get("cwd").and_then(|c| c.as_str());
            for c in content {
                match c.get("type").and_then(|t| t.as_str()) {
                    Some("tool_use") => {
                        if let (Some(id), Some(p)) = (c.get("id").and_then(|x| x.as_str()), pending_of(c, cwd)) { pending.insert(id.to_string(), p); }
                    }
                    Some("tool_result") => {
                        let Some(id) = c.get("tool_use_id").and_then(|x| x.as_str()) else { continue };
                        let Some(p) = pending.remove(id) else { continue };
                        if seen.insert(id.to_string()) { record(&mut s, p, &text_of(c.get("content").unwrap_or(&Value::Null)), min_tokens); }
                    }
                    _ => {}
                }
            }
        }
    }
    s
}
