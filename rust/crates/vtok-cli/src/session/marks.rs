//! Named stretches of work inside an agent's chat: `vtok session start NAME` writes a mark, `stop` another, and a report
//! looks at the calls between them. Marks live in `<state>/marks.jsonl` (a few lines, never trimmed) and belong to a
//! scope: the chat (its session id) or, when no id is known, the directory.
use crate::usage::context::{cwd, norm_path, session_id};
use serde_json::{json, Value};
use std::io::Write;
use std::path::{Path, PathBuf};

const NAME_CHARS: usize = 60;

/// Whose stretch it is.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Scope { pub chat: String, pub cwd: String }

impl Scope {
    /// The chat and directory of this process.
    pub fn current() -> Scope { Scope { chat: session_id().unwrap_or_default(), cwd: norm_path(&cwd()) } }

    /// The same chat, or (when neither side knows its chat) the same directory.
    pub fn same(&self, other: &Scope) -> bool {
        if self.chat.is_empty() && other.chat.is_empty() { self.cwd == other.cwd } else { self.chat == other.chat }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Mark { pub t: u64, pub start: bool, pub name: String, pub scope: Scope }

/// A named stretch: from a start mark to the stop mark, or to the next start in the same scope, or still open.
#[derive(Clone, Debug, PartialEq)]
pub struct Span { pub name: String, pub from: u64, pub to: Option<u64>, pub scope: Scope }

pub fn marks_path() -> Option<PathBuf> { crate::shell::state_dir().map(|d| d.join("marks.jsonl")) }

pub fn read(path: &Path) -> Vec<Mark> {
    let text = String::from_utf8_lossy(&std::fs::read(path).unwrap_or_default()).into_owned();
    text.lines().filter_map(|l| {
        let v: Value = serde_json::from_str(l).ok()?;
        let text = |k: &str| v[k].as_str().unwrap_or("").to_string();
        Some(Mark { t: v["t"].as_u64()?, start: v["op"].as_str()? == "start", name: text("name"), scope: Scope { chat: text("s"), cwd: text("cwd") } })
    }).collect()
}

pub fn append(path: &Path, m: &Mark) -> Result<(), String> {
    if let Some(d) = path.parent() { std::fs::create_dir_all(d).map_err(|e| format!("{}: {e}", d.display()))?; }
    let v = json!({"t": m.t, "op": if m.start { "start" } else { "stop" }, "name": m.name, "s": m.scope.chat, "cwd": m.scope.cwd});
    let mut f = std::fs::OpenOptions::new().create(true).append(true).open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    writeln!(f, "{v}").map_err(|e| e.to_string())
}

/// The stretches the marks describe, oldest first. A start closes the open stretch of its scope, a stop ends it.
pub fn spans(marks: &[Mark]) -> Vec<Span> {
    let mut out: Vec<Span> = Vec::new();
    for m in marks {
        if let Some(open) = out.iter_mut().rev().find(|s| s.to.is_none() && s.scope.same(&m.scope)) { open.to = Some(m.t); }
        if m.start { out.push(Span { name: m.name.clone(), from: m.t, to: None, scope: m.scope.clone() }); }
    }
    out
}

pub fn open_span<'a>(spans: &'a [Span], scope: &Scope) -> Option<&'a Span> { spans.iter().rev().find(|s| s.to.is_none() && s.scope.same(scope)) }

/// The newest stretch of this scope, or the newest with that name.
pub fn pick<'a>(spans: &'a [Span], scope: &Scope, name: Option<&str>) -> Option<&'a Span> {
    spans.iter().rev().find(|s| s.scope.same(scope) && name.map_or(true, |n| s.name == n))
}

/// A name the user can type back: no control characters, at most 60 characters.
pub fn clean_name(raw: &str) -> Result<String, String> {
    let name: String = raw.chars().map(|c| if c.is_control() { ' ' } else { c }).take(NAME_CHARS).collect::<String>().trim().to_string();
    if name.is_empty() { Err("give the session a name: vtok session start NAME".into()) } else { Ok(name) }
}
