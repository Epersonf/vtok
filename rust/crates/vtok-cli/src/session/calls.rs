//! The call-by-call view of a report: every operation with its tokens and a flag, and the ones that did not pay.
use crate::usage::fmt::num;
use crate::usage::kinds::{class, counted, passthrough, saved, Class};
use crate::usage::rec::Rec;

/// Why a call did not help, if it did not: it came out larger, it saved nothing, or vtok printed the output as it was.
pub fn flag(r: &Rec) -> &'static str {
    if class(&r.kind) == Class::NotCounted { return ""; }
    if let Some(p) = passthrough(&r.detail) { return if p == "tiny" { "tiny, printed as it was" } else { "not smaller, printed as it was" }; }
    match saved(&r.kind, r.raw, r.shown) { s if s < 0 => "LARGER than the alternative", 0 => "no gain", _ => "" }
}

/// Calls that cost more than they saved or saved nothing (not counting outputs vtok passed through on purpose), worst first.
pub fn did_not_pay<'a>(recs: &[&'a Rec]) -> Vec<&'a Rec> {
    let mut bad: Vec<&Rec> = recs.iter().copied().filter(|r| class(&r.kind) != Class::NotCounted && passthrough(&r.detail).is_none() && saved(&r.kind, r.raw, r.shown) <= 0).collect();
    bad.sort_by_key(|r| saved(&r.kind, r.raw, r.shown));
    bad
}

fn clock(secs: u64) -> String { format!("+{:02}:{:02}", secs / 60, secs % 60) }

/// Every call, oldest first, up to `limit`; times count from `start`.
pub fn list(recs: &[&Rec], start: u64, limit: usize) -> String {
    let mut out = String::from("calls (time from the start, kind, tokens without vtok -> with it):\n");
    for r in recs.iter().take(limit) {
        let (raw, f) = if class(&r.kind) == Class::NotCounted { ("-".to_string(), "not counted") } else { (num(counted(&r.kind, r.raw) as i64), flag(r)) };
        let note = if f.is_empty() { String::new() } else { format!("   <- {f}") };
        out.push_str(&format!("  {}  {:<7} {:>9} -> {:<9} {}{note}\n", clock(r.t.saturating_sub(start)), r.kind, raw, num(counted(&r.kind, r.shown) as i64), r.detail));
    }
    if recs.len() > limit { out.push_str(&format!("  ... {} more (--limit N)\n", recs.len() - limit)); }
    out
}
