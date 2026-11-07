//! A report as JSON, for scripts and for the A/B harness.
use super::cost::usd;
use super::report::{Opts, Report};
use crate::usage::kinds::{class, class_name, Class};
use crate::usage::tally::{saved_of, totals};
use serde_json::{json, Value};

pub fn json(r: &Report, o: &Opts) -> String {
    let rows: Vec<Value> = r.rows.iter().map(|(k, x)| json!({"kind": k, "class": class_name(class(k)), "calls": x.calls, "raw": x.raw, "shown": x.shown,
        "saved": x.saved, "passed_through": x.passed})).collect();
    let t = totals(&r.rows);
    let cost = r.chat.as_ref().map(|c| json!({"turns": c.turns, "input": c.usage.input, "cache_read": c.usage.cache_read, "cache_write": c.usage.cache_write,
        "output": c.usage.output, "units": c.units, "usd": usd(c.units, o.price_in), "gain_exact_units": c.gain_exact, "gain_upper_units": c.gain_upper,
        "standing_units": c.standing, "net_exact_units": c.net_exact(), "net_all_units": c.net_all(), "net_exact_usd": usd(c.net_exact(), o.price_in),
        "net_all_usd": usd(c.net_all(), o.price_in), "mcp": c.mcp, "model": c.model}));
    json!({"title": r.title, "chat": r.chat_id, "open": r.open, "from": r.from, "to": r.to, "calls": r.recs.len(), "rows": rows, "counted_calls": t.calls,
           "raw": t.raw, "shown": t.shown, "saved_exact": saved_of(&r.rows, Class::Exact), "saved_upper_bound": saved_of(&r.rows, Class::Upper),
           "passed_through": t.passed, "price_in": o.price_in, "cost": cost}).to_string()
}
