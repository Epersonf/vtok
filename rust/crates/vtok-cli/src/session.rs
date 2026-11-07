//! `vtok session start NAME | stop | status | list | report [NAME]`: how much did vtok save in a stretch of work.
//!
//! Every call vtok logs carries the chat's session id (Claude Code's `CLAUDE_CODE_SESSION_ID`, or `VTOK_SESSION` for any
//! other agent), so a chat is a unit with nothing to start. `start NAME` / `stop` mark a named stretch inside it (a task,
//! one request). A report sums the calls with the rules of `vtok gain` (`usage::kinds`) and, for Claude Code, reads the
//! chat's transcript to set them against what the turns cost, including the MCP server's standing cost: the net can be
//! negative. The CLI and the `vtok_session` MCP tool share `execute`.
//!
//! Files: `ask` (the arguments), `marks` (named stretches), `transcript` (Claude Code's file), `cost` (units), `chat`
//! (the transcript side), `report` (building), `render` and `json` (printing), `calls` (call by call), `by_prompt` (the
//! report grouped by what the user typed), `statusline` (the one-line view for a status bar).
mod ask;
mod by_prompt;
mod calls;
mod chat;
mod cost;
mod json;
mod marks;
mod render;
mod report;
mod statusline;
mod transcript;
#[cfg(test)] mod t_by_prompt;
#[cfg(test)] mod t_marks;
#[cfg(test)] mod t_report;
#[cfg(test)] mod t_statusline;
#[cfg(test)] mod t_transcript;

pub use statusline::{init as init_statusline, run as statusline};

use crate::usage::fmt::{dur, now_secs, short, utc};
use crate::usage::filter::Filter;
use ask::{Ask, USAGE};
use marks::{Mark, Scope, Span};
use report::{build, Target};
use serde_json::Value;
use std::path::Path;

pub fn run(args: &[String]) -> i32 {
    match Ask::from_args(args).and_then(|a| execute(&a)) {
        Ok(t) => { println!("{}", t.trim_end()); 0 }
        Err(e) => { eprintln!("{e}"); 2 }
    }
}

/// The MCP tool `vtok_session`: action, name, whole, per_call, by_prompt, price_in, limit, session, all.
pub fn tool(v: &Value) -> Result<String, String> { execute(&Ask::from_json(v)?) }

pub fn execute(a: &Ask) -> Result<String, String> {
    let path = marks::marks_path().ok_or("no state directory (set VTOK_STATE)")?;
    let scope = match &a.chat { Some(c) => Scope { chat: c.clone(), cwd: String::new() }, None => Scope::current() };
    let now = now_secs();
    match a.action.as_str() {
        "start" => start(&path, &scope, a, now),
        "stop" => stop(&path, &scope, a, now),
        "status" => status(&path, &scope, now),
        "list" => Ok(list(&path, &scope, a.all)),
        "report" => report(&path, &scope, a, now),
        other => Err(format!("unknown action {other:?}\n{USAGE}")),
    }
}

fn start(path: &Path, scope: &Scope, a: &Ask, now: u64) -> Result<String, String> {
    let name = marks::clean_name(a.name.as_deref().unwrap_or(""))?;
    marks::append(path, &Mark { t: now, start: true, name: name.clone(), scope: scope.clone() })?;
    let whose = if scope.chat.is_empty() { format!("no chat session id in the environment, so calls are matched by directory ({})", scope.cwd) } else { format!("chat {}", short(&scope.chat)) };
    Ok(format!("session {name:?} started ({whose}). Calls from now on count for it; `vtok session stop` ends it and prints the report."))
}

fn stop(path: &Path, scope: &Scope, a: &Ask, now: u64) -> Result<String, String> {
    let spans = marks::spans(&marks::read(path));
    let open = marks::open_span(&spans, scope).cloned().ok_or("no session is open here (vtok session start NAME)")?;
    marks::append(path, &Mark { t: now, start: false, name: open.name.clone(), scope: scope.clone() })?;
    report(path, scope, &Ask { action: "report".into(), name: Some(open.name), whole: false, ..a.clone() }, now)
}

fn target(spans: &[Span], scope: &Scope, a: &Ask) -> Result<Target, String> {
    let span = if a.whole { None } else { marks::pick(spans, scope, a.name.as_deref()) };
    if let (Some(n), None, false) = (&a.name, span, a.whole) { return Err(format!("no session named {n:?} here; `vtok session list` shows them")); }
    let mut filter = Filter::default();
    if scope.chat.is_empty() { filter.cwd = Some(scope.cwd.clone()); } else { filter.session = Some(scope.chat.clone()); }
    Ok(match span {
        Some(s) => { filter.from = Some(s.from); filter.to = s.to; Target { title: format!("session {:?}", s.name), filter, chat_id: scope.chat.clone(), open: s.to.is_none() } }
        None => Target { title: if scope.chat.is_empty() { "directory" } else { "whole chat" }.into(), filter, chat_id: scope.chat.clone(), open: false },
    })
}

fn report(path: &Path, scope: &Scope, a: &Ask, now: u64) -> Result<String, String> {
    let spans = marks::spans(&marks::read(path));
    let rep = build(&target(&spans, scope, a)?, now);
    Ok(if a.opts.json { json::json(&rep, &a.opts) } else { render::text(&rep, &a.opts) })
}

fn status(path: &Path, scope: &Scope, now: u64) -> Result<String, String> {
    let spans = marks::spans(&marks::read(path));
    let mut f = Filter::default();
    if scope.chat.is_empty() { f.cwd = Some(scope.cwd.clone()); } else { f.session = Some(scope.chat.clone()); }
    let calls = crate::usage::store::read_all().iter().flat_map(|s| s.recs.iter()).filter(|r| f.matches(r, now)).count();
    let whose = if scope.chat.is_empty() { format!("directory {} (no chat session id in the environment)", scope.cwd) } else { format!("chat {}", scope.chat) };
    let open = match marks::open_span(&spans, scope) {
        Some(s) => format!("session {:?} open for {}", s.name, dur(now.saturating_sub(s.from))),
        None => "no session open (vtok session start NAME)".to_string(),
    };
    let log = crate::usage::health().map_or_else(|e| format!("usage log NOT writable: {e}"), |p| format!("usage log {}", p.display()));
    Ok(format!("{whose}\n{open}\n{calls} vtok calls recorded for it\n{log}"))
}

fn list(path: &Path, scope: &Scope, all: bool) -> String {
    let spans = marks::spans(&marks::read(path));
    let mine: Vec<&Span> = spans.iter().filter(|s| all || s.scope.same(scope)).collect();
    if mine.is_empty() { return "no named sessions yet (vtok session start NAME)".into(); }
    mine.iter().map(|s| format!("{:<30} {} .. {}  {}{}", s.name, utc(s.from), s.to.map_or("open".to_string(), utc),
        if s.scope.chat.is_empty() { "dir ".to_string() + &s.scope.cwd } else { format!("chat {}", short(&s.scope.chat)) }, "")).collect::<Vec<_>>().join("\n")
}
