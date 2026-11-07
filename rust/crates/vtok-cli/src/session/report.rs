//! A report: the calls of one chat or named session, summed with the same rules as `gain`, and (for Claude Code chats)
//! joined with the chat's transcript to put them in the cost of the turns around them.
use super::chat::{measure, Chat};
use super::cost::DEFAULT_PRICE_IN;
use super::transcript::{self, Transcript};
use crate::usage::filter::Filter;
use crate::usage::rec::Rec;
use crate::usage::store::read_all;
use crate::usage::tally::{tally, Row};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// What to report on.
pub struct Target {
    pub title: String,
    pub filter: Filter,
    /// The chat whose transcript to read (empty when the chat is unknown).
    pub chat_id: String,
    pub open: bool,
}

/// How to print it.
#[derive(Clone, Debug)]
pub struct Opts { pub price_in: f64, pub per_call: bool, pub by_prompt: bool, pub json: bool, pub limit: Option<usize> }

impl Default for Opts {
    fn default() -> Opts { Opts { price_in: DEFAULT_PRICE_IN, per_call: false, by_prompt: false, json: false, limit: None } }
}

pub struct Report {
    pub title: String,
    pub chat_id: String,
    pub open: bool,
    /// The window, epoch seconds.
    pub from: u64,
    pub to: u64,
    pub recs: Vec<Rec>,
    pub rows: BTreeMap<String, Row>,
    pub chat: Option<Chat>,
    pub transcript: Option<(PathBuf, Transcript)>,
}

/// The window as the transcript's fractional times see it: marks and calls are whole seconds, so the last second is included.
pub fn window(from: u64, to: u64) -> (f64, f64) { (from as f64, to as f64 + 0.999) }

pub fn build(t: &Target, now: u64) -> Report {
    // the MCP server's instructions are in every turn of a chat that has it, whether or not this window used a tool
    let mut mcp = false;
    let mut recs: Vec<Rec> = Vec::new();
    for r in read_all().into_iter().flat_map(|s| s.recs) {
        mcp |= !t.chat_id.is_empty() && r.session == t.chat_id && r.via == "mcp";
        if t.filter.matches(&r, now) { recs.push(r); }
    }
    recs.sort_by_key(|r| r.t);
    let transcript = (!t.chat_id.is_empty()).then(|| transcript::find(&t.chat_id)).flatten().map(|p| { let tr = transcript::load(&p); (p, tr) });
    let turns = transcript.as_ref().map_or(&[][..], |(_, tr)| tr.turns.as_slice());
    // a named stretch has its own ends; the whole chat runs from its first turn or call to its last (to now while open)
    let first = turns.first().map(|x| x.t as u64).into_iter().chain(recs.first().map(|r| r.t)).min();
    let last = turns.last().map(|x| x.t.ceil() as u64).into_iter().chain(recs.last().map(|r| r.t)).max();
    let from = t.filter.from.or(first).unwrap_or(now);
    let to = t.filter.to.or(if t.open { Some(now) } else { last }).unwrap_or(now);
    let rows = tally(recs.iter());
    let (wf, wt) = window(from, to);
    let chat = transcript.as_ref().map(|(_, tr)| measure(tr, &recs.iter().collect::<Vec<_>>(), wf, wt, mcp));
    Report { title: t.title.clone(), chat_id: t.chat_id.clone(), open: t.open, from, to, recs, rows, chat, transcript }
}
