//! What a caller asks of `vtok session`, from the command line or from the MCP tool.
use super::report::Opts;
use crate::usage::filter::resolve_session;
use serde_json::Value;

pub const USAGE: &str = "usage: vtok session start NAME | stop | status | list [--all] | report [NAME]
  report [NAME] [--whole] [--per-call] [--by-prompt] [--limit N] [--price-in USD_PER_M] [--json] [--session ID]
  NAME is a stretch of work you named with start; without it the newest one is reported, and without any the whole chat.";

#[derive(Clone, Debug)]
pub struct Ask { pub action: String, pub name: Option<String>, pub opts: Opts, pub whole: bool, pub chat: Option<String>, pub all: bool }

impl Ask {
    pub fn from_args(args: &[String]) -> Result<Ask, String> {
        let action = args.first().cloned().ok_or_else(|| USAGE.to_string())?;
        let mut a = Ask { action, name: None, opts: Opts::default(), whole: false, chat: None, all: false };
        let mut i = 1;
        while i < args.len() {
            let x = args[i].as_str();
            let mut value = || { i += 1; args.get(i).cloned().ok_or_else(|| format!("{x} needs a value")) };
            match x {
                "--per-call" => a.opts.per_call = true,
                "--by-prompt" => a.opts.by_prompt = true,
                "--json" => a.opts.json = true,
                "--whole" => a.whole = true,
                "--all" => a.all = true,
                "--limit" => a.opts.limit = Some(value()?.parse().map_err(|_| "--limit wants a number".to_string())?),
                "--price-in" => a.opts.price_in = value()?.parse().map_err(|_| "--price-in wants a number (US$ per million input tokens)".to_string())?,
                "--session" => a.chat = Some(resolve_session(&value()?)?),
                s if s.starts_with("--") => return Err(format!("unknown option {s}\n{USAGE}")),
                s => a.name = Some(match a.name.take() { Some(n) => format!("{n} {s}"), None => s.to_string() }),
            }
            i += 1;
        }
        Ok(a)
    }

    pub fn from_json(v: &Value) -> Result<Ask, String> {
        let flag = |k: &str| v.get(k).and_then(|x| x.as_bool()).unwrap_or(false);
        let mut a = Ask { action: v.get("action").and_then(|x| x.as_str()).unwrap_or("report").to_string(), name: v.get("name").and_then(|x| x.as_str()).map(String::from),
                          opts: Opts::default(), whole: flag("whole"), chat: None, all: flag("all") };
        a.opts.per_call = flag("per_call");
        a.opts.by_prompt = flag("by_prompt");
        if let Some(p) = v.get("price_in").and_then(|x| x.as_f64()) { a.opts.price_in = p; }
        if let Some(n) = v.get("limit").and_then(|x| x.as_u64()) { a.opts.limit = Some(n as usize); }
        if let Some(s) = v.get("session").and_then(|x| x.as_str()) { a.chat = Some(resolve_session(s)?); }
        Ok(a)
    }
}
