//! The chat side of a report: what the turns of a window used, and what vtok's calls were worth in them. A call that kept
//! tokens out of the context saves a cache write and a read for every later turn up to the next compaction (which
//! throws the old tool results away); the MCP server's instructions cost a read on every turn of the window.
use super::cost::{ride, standing, units};
use super::transcript::{Transcript, Usage};
use crate::usage::kinds::{class, saved, Class};
use crate::usage::rec::Rec;

#[derive(Debug, Default, PartialEq)]
pub struct Chat {
    pub turns: usize,
    pub usage: Usage,
    /// What the window's turns cost, in cache-read units.
    pub units: f64,
    /// Worth of the calls whose comparison is exact (`vtok run`), and of those compared with a whole file.
    pub gain_exact: f64,
    pub gain_upper: f64,
    /// Cost of the MCP server's instructions over the window's turns (0 when no MCP call was seen).
    pub standing: f64,
    pub mcp: bool,
    pub model: Option<String>,
}

impl Chat {
    /// The floor: only exact calls, minus the standing cost.
    pub fn net_exact(&self) -> f64 { self.gain_exact - self.standing }
    /// The ceiling: every counted call against its comparison, minus the standing cost.
    pub fn net_all(&self) -> f64 { self.gain_exact + self.gain_upper - self.standing }
}

/// `from` and `to` bound the window (epoch seconds); `recs` are the vtok calls inside it; `mcp` says the chat has the MCP
/// server (it was used at some point), so its instructions were in every turn of the window.
pub fn measure(tr: &Transcript, recs: &[&Rec], from: f64, to: f64, mcp: bool) -> Chat { measure_in(tr, recs, from, to, to, mcp) }

/// Like `measure`, but a call's saving rides the turns up to `ride_to`, which may be later than the end of the window whose
/// cost is counted (a prompt's calls pay off in the prompts that follow).
pub fn measure_in(tr: &Transcript, recs: &[&Rec], from: f64, to: f64, ride_to: f64, mcp: bool) -> Chat {
    let window: Vec<_> = tr.turns.iter().filter(|t| t.t >= from && t.t <= to).collect();
    let mut usage = Usage::default();
    for t in &window {
        usage = Usage { input: usage.input + t.usage.input, cache_read: usage.cache_read + t.usage.cache_read, cache_write: usage.cache_write + t.usage.cache_write, output: usage.output + t.usage.output };
    }
    let mut chat = Chat { turns: window.len(), usage, units: units(&usage), model: tr.model.clone(), mcp, ..Default::default() };
    for r in recs {
        let worth = saved(&r.kind, r.raw, r.shown);
        if worth == 0 { continue; }
        let at = r.t as f64;
        let first = tr.turns.partition_point(|t| t.t <= at);
        // a saved token is worth a read per turn it would have ridden, up to the next compaction or the end of the window
        let compaction = tr.compactions.iter().copied().find(|&c| c > at).unwrap_or(f64::INFINITY);
        let last = tr.turns.partition_point(|t| t.t < compaction).min(tr.turns.partition_point(|t| t.t <= ride_to));
        let gain = worth as f64 * ride(last.saturating_sub(first));
        match class(&r.kind) { Class::Exact => chat.gain_exact += gain, Class::Upper => chat.gain_upper += gain, Class::NotCounted => {} }
    }
    let first_turn_in = tr.turns.first().map_or(false, |t| t.t >= from && t.t <= to);
    if chat.mcp { chat.standing = standing(chat.turns, first_turn_in); }
    chat
}
