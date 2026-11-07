//! `vtok gain`: what the recorded calls saved, by tool, with the limits of each number stated (see `kinds`).
use super::filter::Filter;
use super::fmt::{now_secs, num};
use super::kinds::{basis, class, class_name, counted, passthrough, saved, Class};
use super::store::{read_all, Source};
use serde_json::{json, Value};
use std::collections::BTreeMap;

/// Per kind: calls, tokens as counted (see `kinds`), their difference, and the calls vtok printed as they were.
#[derive(Default, Clone, Copy)]
struct Row { calls: u64, raw: u64, shown: u64, saved: i64, passed: u64 }

/// Sums per kind over the records the filter lets through, and how many untagged older records it left out.
fn tally(sources: &[Source], f: &Filter, now: u64) -> (BTreeMap<String, Row>, u64) {
    let wide = Filter { legacy: true, ..f.clone() };
    let mut rows: BTreeMap<String, Row> = BTreeMap::new();
    let mut skipped = 0;
    for r in sources.iter().flat_map(|s| s.recs.iter()) {
        if f.matches(r, now) {
            let e = rows.entry(r.kind.clone()).or_default();
            e.calls += 1;
            e.raw += counted(&r.kind, r.raw);
            e.shown += counted(&r.kind, r.shown);
            e.saved += saved(&r.kind, r.raw, r.shown);
            e.passed += passthrough(&r.detail).is_some() as u64;
        } else if !r.tagged && !f.legacy && wide.matches(r, now) { skipped += 1; }
    }
    (rows, skipped)
}

fn saved_of(rows: &BTreeMap<String, Row>, c: Class) -> i64 {
    rows.iter().filter(|(k, _)| class(k) == c).map(|(_, r)| r.saved).sum()
}

fn totals(rows: &BTreeMap<String, Row>) -> Row {
    rows.iter().filter(|(k, _)| class(k) != Class::NotCounted).fold(Row::default(), |a, (_, r)| Row { calls: a.calls + r.calls, raw: a.raw + r.raw, shown: a.shown + r.shown, saved: a.saved + r.saved, passed: a.passed + r.passed })
}

pub(super) fn render(sources: &[Source], f: &Filter, now: u64, as_json: bool) -> String {
    let (rows, skipped) = tally(sources, f, now);
    let (t, exact, upper) = (totals(&rows), saved_of(&rows, Class::Exact), saved_of(&rows, Class::Upper));
    if as_json {
        let list: Vec<Value> = rows.iter().map(|(k, r)| json!({"kind": k, "class": class_name(class(k)), "calls": r.calls, "raw": r.raw, "shown": r.shown,
            "saved": r.saved, "passed_through": r.passed})).collect();
        let logs: Vec<Value> = sources.iter().map(|s| json!({"path": s.path.display().to_string(), "records": s.recs.len()})).collect();
        return json!({"days": f.days, "rows": list, "calls": t.calls, "raw": t.raw, "shown": t.shown, "saved_exact": exact, "saved_upper_bound": upper,
                      "passed_through": t.passed, "legacy_skipped": skipped, "logs": logs}).to_string();
    }
    let paths: Vec<String> = sources.iter().map(|s| format!("{} ({} records)", s.path.display(), s.recs.len())).collect();
    if rows.is_empty() {
        let hint = if skipped > 0 { format!("; {} older records without a session tag were left out, --legacy adds them", num(skipped as i64)) } else { String::new() };
        return format!("no usage recorded ({}){hint}\nlooked in: {}", f.describe(), if paths.is_empty() { "no log file".into() } else { paths.join(", ") });
    }
    let mut out = format!("vtok usage, {}\nread: {}\n\n", f.describe(), paths.join("; "));
    out.push_str(&format!("{:<8} {:>6} {:>12} {:>12} {:>12} {:>7}  {}\n", "tool", "calls", "raw tokens", "shown", "saved", "saved%", "raw is"));
    for (k, r) in &rows {
        if class(k) == Class::NotCounted {
            out.push_str(&format!("{:<8} {:>6} {:>12} {:>12} {:>12} {:>7}  {}\n", k, r.calls, "-", num(r.shown as i64), "-", "-", basis(k)));
        } else {
            let pct = if r.raw > 0 { 100.0 * r.saved as f64 / r.raw as f64 } else { 0.0 };
            out.push_str(&format!("{:<8} {:>6} {:>12} {:>12} {:>12} {:>6.1}%  {}\n", k, r.calls, num(r.raw as i64), num(r.shown as i64), num(r.saved), pct, basis(k)));
        }
    }
    out.push_str(&format!("\ncounted: {} calls, {} tokens without vtok, {} with it; saved {} (exact {}, upper bound {}).\n", num(t.calls as i64), num(t.raw as i64), num(t.shown as i64), num(exact + upper), num(exact), num(upper)));
    if t.passed > 0 { out.push_str(&format!("{} of the counted calls were printed as they were (a tiny output, or a view that was not smaller).\n", num(t.passed as i64))); }
    out.push_str("A saving is tokens that were not sent: an upper bound on money saved, because it cannot see the extra turns a compact view may cause. Only an A/B run (bench/loop) answers that.\n");
    if skipped > 0 { out.push_str(&format!("{} older records without a session tag were left out (they include test and benchmark runs); --legacy adds them.\n", num(skipped as i64))); }
    out
}

pub fn gain_text(f: &Filter, as_json: bool) -> Result<String, String> {
    if crate::shell::state_dirs().is_empty() { return Err("no state directory (set HOME or VTOK_STATE)".into()); }
    Ok(render(&read_all(), f, now_secs(), as_json))
}

/// The MCP tool: `days`, `since`, `session` (an id or "current"), `cwd`, `legacy`.
pub fn gain_tool(args: &Value) -> Result<String, String> { gain_text(&Filter::from_json(args, Some(30))?, false) }

/// One short line for a status bar: `vtok saved 12.3k tok (41%) / 7d`. Empty when nothing was recorded.
pub fn line(f: &Filter) -> String {
    let Ok(j) = gain_text(f, true) else { return String::new() };
    let Ok(v) = serde_json::from_str::<Value>(&j) else { return String::new() };
    let (raw, shown) = (v["raw"].as_i64().unwrap_or(0), v["shown"].as_i64().unwrap_or(0));
    if raw <= 0 { return String::new(); }
    let saved = raw - shown;
    format!("vtok saved {} tok ({:.0}%) / {}", super::fmt::tok(saved), 100.0 * saved as f64 / raw as f64, f.describe())
}

pub fn gain(args: &[String]) -> i32 {
    let f = match Filter::from_args(args, Some(30)) { Ok(f) => f, Err(e) => { eprintln!("{e}"); return 2 } };
    if crate::args::has(args, "--line") { println!("{}", line(&f)); return 0; }
    match gain_text(&f, crate::args::has(args, "--json")) {
        Ok(t) => { println!("{}", t.trim_end()); 0 }
        Err(e) => { eprintln!("{e}"); 1 }
    }
}
