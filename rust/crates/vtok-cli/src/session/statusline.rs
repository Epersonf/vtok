//! `vtok statusline`: one short line for an agent's status bar, from the usage log alone (no transcript, so it is instant):
//! what vtok saved in this chat, or in the named stretch that is open. Claude Code runs the command and sends its status
//! JSON on stdin; the chat is its `session_id`. It prints nothing when there is nothing to say and never fails.
//! `vtok init --claude-statusline SETTINGS` registers it, only when the settings have no status line yet.
use super::marks::{self, Scope};
use crate::usage::context::norm_path;
use crate::usage::filter::Filter;
use crate::usage::fmt::{now_secs, tok};
use crate::usage::kinds::Class;
use crate::usage::rec::Rec;
use crate::usage::tally::{saved_of, tally};
use serde_json::{json, Value};
use std::io::{IsTerminal, Read};

/// `vtok saved 5.1k-11.1k tok · 14 calls` (exact calls to exact plus whole-file calls), `vtok[login] ...` in a named stretch.
pub fn line(recs: &[&Rec], open: Option<&str>) -> String {
    if recs.is_empty() { return String::new(); }
    let rows = tally(recs.iter().copied());
    let (exact, upper) = (saved_of(&rows, Class::Exact), saved_of(&rows, Class::Upper));
    let amount = if upper <= 0 { tok(exact) } else if exact <= 0 { format!("up to {}", tok(upper)) } else { format!("{}-{}", tok(exact), tok(exact + upper)) };
    format!("vtok{} saved {amount} tok · {} calls", open.map_or(String::new(), |n| format!("[{n}]")), recs.len())
}

/// The chat and the directory the agent's status JSON names (empty when it names none, or when nothing was piped in).
fn input() -> (String, String) {
    let mut s = String::new();
    if !std::io::stdin().is_terminal() { let _ = std::io::stdin().read_to_string(&mut s); }
    let v: Value = serde_json::from_str(&s).unwrap_or(Value::Null);
    let text = |x: Option<&str>| x.unwrap_or("").to_string();
    (text(v["session_id"].as_str()), text(v["workspace"]["current_dir"].as_str().or(v["cwd"].as_str())))
}

pub fn run(_args: &[String]) -> i32 {
    let (chat, cwd) = input();
    let mut scope = Scope::current();
    if !chat.is_empty() { scope = Scope { chat, cwd: String::new() }; } else if scope.chat.is_empty() && !cwd.is_empty() { scope.cwd = norm_path(&cwd); }
    let Some(path) = marks::marks_path() else { return 0 };
    let spans = marks::spans(&marks::read(&path));
    let open = marks::open_span(&spans, &scope);
    let mut f = Filter { from: open.map(|s| s.from), ..Filter::default() };
    if scope.chat.is_empty() { f.cwd = Some(scope.cwd.clone()); } else { f.session = Some(scope.chat.clone()); }
    let (all, now) = (crate::usage::store::read_all(), now_secs());
    let recs: Vec<&Rec> = all.iter().flat_map(|s| s.recs.iter()).filter(|r| f.matches(r, now)).collect();
    let text = line(&recs, open.map(|s| s.name.as_str()));
    if !text.is_empty() { println!("{text}"); }
    0
}

/// What `init --claude-statusline` found in the settings.
#[derive(Debug, PartialEq)]
pub enum Installed { Added(String), Present(String) }

pub fn install_with(path: &str, command: &str) -> Result<Installed, String> {
    let mut v: Value = match std::fs::read_to_string(path) {
        Ok(t) if !t.trim().is_empty() => serde_json::from_str(&t).map_err(|e| format!("{path}: not valid JSON ({e}); left untouched"))?,
        _ => json!({}),
    };
    let obj = v.as_object_mut().ok_or_else(|| format!("{path}: not a JSON object; left untouched"))?;
    if let Some(cur) = obj.get("statusLine") { return Ok(Installed::Present(cur["command"].as_str().unwrap_or("").to_string())); }
    obj.insert("statusLine".into(), json!({"type": "command", "command": command, "padding": 0}));
    if let Some(d) = std::path::Path::new(path).parent() { if !d.as_os_str().is_empty() { std::fs::create_dir_all(d).map_err(|e| e.to_string())?; } }
    std::fs::write(path, serde_json::to_string_pretty(&v).unwrap() + "\n").map_err(|e| format!("{path}: {e}"))?;
    Ok(Installed::Added(command.to_string()))
}

/// True for a command that runs this program's `statusline`: its program is named `vtok...` and it ends in ` statusline`.
fn is_ours(command: &str) -> bool {
    let c = command.trim();
    let prog = match c.strip_prefix('"') { Some(r) => r.split('"').next().unwrap_or(""), None => c.split(' ').next().unwrap_or("") };
    prog.rsplit(['/', '\\']).next().unwrap_or("").starts_with("vtok") && c.ends_with(" statusline")
}

/// `vtok init --claude-statusline SETTINGS`.
pub fn init(path: Option<&String>) -> i32 {
    let Some(path) = path else { eprintln!("usage: vtok init --claude-statusline SETTINGS.json   (e.g. ~/.claude/settings.json; adds the vtok line only when no status line is set)"); return 2 };
    match install_with(path, &format!("{} statusline", crate::hook::binary())) {
        Ok(Installed::Added(c)) => { println!("{path}: status line added as `{c}` (what vtok saved in this chat; empty until vtok is used)"); 0 }
        Ok(Installed::Present(c)) if is_ours(&c) => { println!("{path}: already up to date"); 0 }
        Ok(Installed::Present(c)) => { println!("{path}: a status line is already set (`{c}`); left as it is. To show the vtok line there, run `vtok statusline` in that script (it reads the same JSON on stdin)"); 0 }
        Err(e) => { eprintln!("{e}"); 1 }
    }
}
