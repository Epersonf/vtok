//! The report by prompt: what the user typed starts a stretch that lasts until the next prompt, and each stretch gets what
//! its turns cost and what vtok's calls in it were worth. A saving still counts up to the end of the whole report (a call
//! pays off in the prompts that follow it), so the figures of one prompt are not a verdict on that prompt alone.
use super::chat::{measure_in, Chat};
use super::cost::usd;
use super::report::{window, Opts, Report};
use super::transcript::Transcript;
use crate::usage::fmt::num;
use crate::usage::kinds::{class, saved, Class};
use crate::usage::rec::Rec;
use serde_json::{json, Value};

const DEFAULT_ROWS: usize = 15;

pub struct Line {
    pub at: f64,
    pub text: String,
    pub calls: usize,
    /// Tokens its counted calls saved.
    pub tokens: i64,
    pub chat: Chat,
}

pub fn lines(tr: &Transcript, recs: &[&Rec], from: f64, to: f64, mcp: bool) -> Vec<Line> {
    let mut out = Vec::new();
    for (i, p) in tr.prompts.iter().enumerate() {
        let end = tr.prompts.get(i + 1).map_or(f64::INFINITY, |n| n.t);
        let (a, b) = (p.t.max(from), (end - 1e-3).min(to));
        if a > b { continue; }
        let mine: Vec<&Rec> = recs.iter().copied().filter(|r| (r.t as f64) >= a && (r.t as f64) <= b).collect();
        let chat = measure_in(tr, &mine, a, b, to, mcp);
        if chat.turns == 0 && mine.is_empty() { continue; }
        let tokens = mine.iter().filter(|r| class(&r.kind) != Class::NotCounted).map(|r| saved(&r.kind, r.raw, r.shown)).sum();
        out.push(Line { at: a, text: p.text.clone(), calls: mine.len(), tokens, chat });
    }
    out
}

/// The prompts of a report's window; empty without a transcript.
pub fn of(r: &Report) -> Vec<Line> {
    match (&r.transcript, &r.chat) {
        (Some((_, tr)), Some(c)) => { let (from, to) = window(r.from, r.to); lines(tr, &r.recs.iter().collect::<Vec<_>>(), from, to, c.mcp) }
        _ => Vec::new(),
    }
}

fn clock(secs: f64) -> String {
    let s = secs.max(0.0) as u64;
    if s < 3600 { format!("+{:02}:{:02}", s / 60, s % 60) } else { format!("+{}h{:02}", s / 3600, s % 3600 / 60) }
}

fn dollars(units: f64, o: &Opts) -> String { format!("{}US${:.3}", if units < 0.0 { "-" } else { "" }, usd(units.abs(), o.price_in)) }

/// The dearest prompts first, with their cost, their calls and the net of those calls (floor and ceiling).
pub fn table(mut lines: Vec<Line>, start: f64, o: &Opts) -> String {
    let total = lines.len();
    lines.sort_by(|a, b| b.chat.units.total_cmp(&a.chat.units));
    let shown = o.limit.unwrap_or(DEFAULT_ROWS).min(total);
    let mut s = format!("by prompt, dearest first ({shown} of {total}); a saving counts up to the end of the window, net = worth of the calls - MCP instructions\n");
    s.push_str("  at        turns        cost  calls  saved tok   net exact     net all  prompt\n");
    for l in lines.iter().take(shown) {
        s.push_str(&format!("  {:<8} {:>5} {:>11} {:>6} {:>10} {:>11} {:>11}  {}\n", clock(l.at - start), l.chat.turns, dollars(l.chat.units, o), l.calls, num(l.tokens),
                            dollars(l.chat.net_exact(), o), dollars(l.chat.net_all(), o), l.text));
    }
    s
}

pub fn json_of(lines: &[Line], start: f64, o: &Opts) -> Vec<Value> {
    lines.iter().map(|l| json!({"at": l.at - start, "prompt": l.text, "turns": l.chat.turns, "units": l.chat.units, "usd": usd(l.chat.units, o.price_in), "calls": l.calls,
        "saved_tokens": l.tokens, "gain_exact_units": l.chat.gain_exact, "gain_upper_units": l.chat.gain_upper, "standing_units": l.chat.standing,
        "net_exact_units": l.chat.net_exact(), "net_all_units": l.chat.net_all()})).collect()
}
