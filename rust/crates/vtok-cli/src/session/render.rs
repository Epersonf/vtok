//! The text of a report.
use super::by_prompt;
use super::calls::{did_not_pay, list};
use super::cost::{usd, READ, STANDING_TOKENS};
use super::report::{Opts, Report};
use crate::usage::fmt::{dur, num, short};
use crate::usage::kinds::{class, Class};
use crate::usage::rec::Rec;
use crate::usage::tally::{saved_of, table, totals};

const FOOTER: &str = "This counts tokens that were not sent. It cannot see the extra turns a compact view may cause, nor calls made by subagents (their context is separate); only an A/B run (bench/loop) says what vtok really saves.\n";

fn money(units: f64, o: &Opts) -> String { format!("{}US${:.2}", if units < 0.0 { "-" } else { "" }, usd(units.abs(), o.price_in)) }

fn signed(units: f64) -> String { format!("{}{}", if units >= 0.0 { "+" } else { "-" }, num(units.abs().round() as i64)) }

pub fn text(r: &Report, o: &Opts) -> String {
    let t = totals(&r.rows);
    let (exact, upper) = (saved_of(&r.rows, Class::Exact), saved_of(&r.rows, Class::Upper));
    let chat = if r.chat_id.is_empty() { String::new() } else { format!(" · chat {}", short(&r.chat_id)) };
    let mut out = format!("vtok {}{}{} · {} · {} calls\n", r.title, if r.open { " (open)" } else { "" }, chat, dur(r.to.saturating_sub(r.from)), num(r.recs.len() as i64));
    if r.recs.is_empty() {
        out.push_str("no vtok calls recorded here yet. Calls are tagged with the chat's session id; an MCP server started before the tags existed does not record at all, so reconnect it in a new session.\n");
        return out;
    }
    out.push('\n');
    out.push_str(&table(&r.rows));
    out.push_str(&format!("\ncounted: {} calls, {} tokens without vtok, {} with it; saved {} (exact {}, upper bound {}).\n", num(t.calls as i64), num(t.raw as i64), num(t.shown as i64), num(exact + upper), num(exact), num(upper)));
    let refs: Vec<&Rec> = r.recs.iter().collect();
    let bad = did_not_pay(&refs);
    let counted = r.recs.iter().filter(|x| class(&x.kind) != Class::NotCounted).count();
    if t.passed > 0 { out.push_str(&format!("{} calls were printed as they were (a tiny output, or a view that was not smaller).\n", num(t.passed as i64))); }
    if !bad.is_empty() {
        let shown: Vec<String> = bad.iter().take(5).map(|x| format!("{} {} {} -> {}", x.kind, x.detail, num(x.raw as i64), num(x.shown as i64))).collect();
        out.push_str(&format!("did not pay ({} of {} counted calls): {}\n", bad.len(), counted, shown.join(" · ")));
    }
    if o.per_call { out.push('\n'); out.push_str(&list(&refs, r.from, o.limit.unwrap_or(40))); }
    out.push('\n');
    out.push_str(&chat_section(r, o));
    if o.by_prompt { out.push('\n'); out.push_str(&prompts(r, o)); }
    out.push('\n');
    out.push_str(FOOTER);
    out
}

fn prompts(r: &Report, o: &Opts) -> String {
    let lines = by_prompt::of(r);
    if lines.is_empty() { "No prompts to group by: that needs the chat's transcript.\n".into() } else { by_prompt::table(lines, r.from as f64, o) }
}

fn chat_section(r: &Report, o: &Opts) -> String {
    let Some(c) = &r.chat else {
        return if r.chat_id.is_empty() { "No chat session id here, so no transcript to compare with: tokens only.\n".into() }
        else { format!("No transcript found for chat {} (Claude Code keeps it under ~/.claude/projects): tokens only.\n", short(&r.chat_id)) };
    };
    let (file, _) = r.transcript.as_ref().expect("a measured chat has a transcript");
    let name = file.file_name().map_or(String::new(), |n| n.to_string_lossy().into_owned());
    let mut s = format!("chat transcript {name} (main conversation): {} turns in the window\n", num(c.turns as i64));
    s.push_str(&format!("  cache read {} · cache write {} · output {} · input {} ≈ {} at input US${:.2}/M{}\n", num(c.usage.cache_read as i64), num(c.usage.cache_write as i64), num(c.usage.output as i64), num(c.usage.input as i64), money(c.units, o), o.price_in, c.model.as_ref().map_or(String::new(), |m| format!(" (model {m})"))));
    s.push_str(&format!("worth, in cache-read units (1 unit = one cached token read again = US${:.2}/M):\n", usd(1e6, o.price_in)));
    s.push_str(&format!("  exact calls (vtok run)           {:>12}  {}\n", signed(c.gain_exact), money(c.gain_exact, o)));
    s.push_str(&format!("  whole-file calls (upper bound)   {:>12}  {}\n", signed(c.gain_upper), money(c.gain_upper, o)));
    let why = if c.mcp { format!("{} tokens x {} turns", STANDING_TOKENS as i64, c.turns) } else { "no MCP call seen; if the server is registered it costs ".to_string() + &format!("{} tokens x {} turns = {} units", STANDING_TOKENS as i64, c.turns, num((STANDING_TOKENS * READ * c.turns as f64) as i64)) };
    s.push_str(&format!("  MCP instructions ({why})  {:>12}\n", signed(-c.standing)));
    let pct = |n: f64| if c.units > 0.0 { 100.0 * n / c.units } else { 0.0 };
    s.push_str(&format!("  net: {} units ({}, {:.1}% of the window) counting exact calls only, {} ({}, {:.1}%) with the upper-bound ones\n", signed(c.net_exact()), money(c.net_exact(), o), pct(c.net_exact()), signed(c.net_all()), money(c.net_all(), o), pct(c.net_all())));
    s
}
