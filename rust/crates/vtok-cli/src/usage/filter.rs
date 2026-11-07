//! Which records a report looks at: how far back, which chat, which directory, which stretch of time.
use super::context::{norm_path, session_id, within};
use super::fmt::{dur, short};
use super::rec::Rec;
use serde_json::Value;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Filter {
    /// Only the last N days.
    pub days: Option<u64>,
    /// Only the last N seconds.
    pub since: Option<u64>,
    /// Only calls tagged with this chat session id.
    pub session: Option<String>,
    /// Only calls made inside this directory.
    pub cwd: Option<String>,
    /// Only this stretch of time (epoch seconds, both ends included): a named session.
    pub from: Option<u64>,
    pub to: Option<u64>,
    /// Include records written before the tags existed.
    pub legacy: bool,
}

/// `90s`, `30m`, `2h`, `7d` or plain seconds.
pub fn parse_duration(s: &str) -> Option<u64> {
    let s = s.trim();
    let (digits, unit) = match s.chars().last()? { c if c.is_ascii_digit() => (s, 1), 's' => (&s[..s.len() - 1], 1), 'm' => (&s[..s.len() - 1], 60), 'h' => (&s[..s.len() - 1], 3600), 'd' => (&s[..s.len() - 1], 86_400), _ => return None };
    digits.trim().parse::<u64>().ok().map(|n| n * unit)
}

/// `current` is the chat this process runs in; anything else is taken as an id.
pub fn resolve_session(value: &str) -> Result<String, String> {
    if value == "current" { return session_id().ok_or_else(|| "no chat session id in the environment (set VTOK_SESSION or pass --session ID)".to_string()); }
    Ok(value.trim().to_string())
}

impl Filter {
    /// From command-line flags (`--days`, `--since`, `--session`, `--cwd`, `--legacy`); `default_days` applies when nothing narrows the view.
    pub fn from_args(args: &[String], default_days: Option<u64>) -> Result<Filter, String> {
        let flag = |n: &str| crate::args::flag(args, n);
        let mut f = Filter { legacy: crate::args::has(args, "--legacy"), cwd: flag("--cwd"), ..Default::default() };
        if let Some(d) = flag("--days") { f.days = Some(d.parse().map_err(|_| format!("--days wants a number, got {d:?}"))?); }
        if let Some(s) = flag("--since") { f.since = Some(parse_duration(&s).ok_or_else(|| format!("--since wants 90s, 30m, 2h or 7d, got {s:?}"))?); }
        if let Some(s) = flag("--session") { f.session = Some(resolve_session(&s)?); }
        f.settle(default_days);
        Ok(f)
    }

    /// From the arguments of an MCP tool call (`days`, `since`, `session`, `cwd`, `legacy`).
    pub fn from_json(a: &Value, default_days: Option<u64>) -> Result<Filter, String> {
        let text = |k: &str| a.get(k).and_then(|v| v.as_str()).map(String::from);
        let mut f = Filter { legacy: a.get("legacy").and_then(|v| v.as_bool()).unwrap_or(false), cwd: text("cwd"), days: a.get("days").and_then(|v| v.as_u64()), ..Default::default() };
        if let Some(s) = text("since") { f.since = Some(parse_duration(&s).ok_or_else(|| format!("since wants 90s, 30m, 2h or 7d, got {s:?}"))?); }
        if let Some(s) = text("session") { f.session = Some(resolve_session(&s)?); }
        f.settle(default_days);
        Ok(f)
    }

    fn settle(&mut self, default_days: Option<u64>) {
        if self.days.is_none() && self.since.is_none() && self.session.is_none() && self.cwd.is_none() { self.days = default_days; }
    }

    pub fn matches(&self, r: &Rec, now: u64) -> bool {
        if !r.tagged && !self.legacy { return false; }
        if self.days.map_or(false, |d| r.t < now.saturating_sub(d.saturating_mul(86_400))) { return false; }
        if self.since.map_or(false, |s| r.t < now.saturating_sub(s)) { return false; }
        if self.from.map_or(false, |f| r.t < f) || self.to.map_or(false, |t| r.t > t) { return false; }
        if self.session.as_ref().map_or(false, |s| &r.session != s) { return false; }
        if self.cwd.as_ref().map_or(false, |c| !within(&norm_path(&r.cwd), &norm_path(c))) { return false; }
        true
    }

    /// The scope in words, for report headers.
    pub fn describe(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        if let Some(s) = &self.session { parts.push(format!("chat {}", short(s))); }
        if let Some(d) = self.days { parts.push(format!("last {d} days")); }
        if let Some(s) = self.since { parts.push(format!("last {}", dur(s))); }
        if let Some(c) = &self.cwd { parts.push(format!("inside {c}")); }
        if parts.is_empty() { parts.push("all time".into()); }
        parts.join(", ")
    }
}
