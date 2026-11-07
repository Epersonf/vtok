//! Sums of recorded calls per kind, as counted (see `kinds`), and the table both `gain` and the session report print.
use super::fmt::num;
use super::kinds::{basis, class, counted, passthrough, saved, Class};
use super::rec::Rec;
use std::collections::BTreeMap;

/// Per kind: calls, tokens as counted, their difference, and the calls vtok printed as they were.
#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub struct Row { pub calls: u64, pub raw: u64, pub shown: u64, pub saved: i64, pub passed: u64 }

pub fn tally<'a>(recs: impl Iterator<Item = &'a Rec>) -> BTreeMap<String, Row> {
    let mut rows: BTreeMap<String, Row> = BTreeMap::new();
    for r in recs {
        let e = rows.entry(r.kind.clone()).or_default();
        e.calls += 1;
        e.raw += counted(&r.kind, r.raw);
        e.shown += counted(&r.kind, r.shown);
        e.saved += saved(&r.kind, r.raw, r.shown);
        e.passed += passthrough(&r.detail).is_some() as u64;
    }
    rows
}

pub fn saved_of(rows: &BTreeMap<String, Row>, c: Class) -> i64 {
    rows.iter().filter(|(k, _)| class(k) == c).map(|(_, r)| r.saved).sum()
}

/// Totals over the kinds that are counted.
pub fn totals(rows: &BTreeMap<String, Row>) -> Row {
    rows.iter().filter(|(k, _)| class(k) != Class::NotCounted).fold(Row::default(), |a, (_, r)| Row {
        calls: a.calls + r.calls, raw: a.raw + r.raw, shown: a.shown + r.shown, saved: a.saved + r.saved, passed: a.passed + r.passed })
}

/// One line per kind, with what its `raw` stands for.
pub fn table(rows: &BTreeMap<String, Row>) -> String {
    let mut out = format!("{:<8} {:>6} {:>12} {:>12} {:>12} {:>7}  {}\n", "tool", "calls", "raw tokens", "shown", "saved", "saved%", "raw is");
    for (k, r) in rows {
        if class(k) == Class::NotCounted {
            out.push_str(&format!("{:<8} {:>6} {:>12} {:>12} {:>12} {:>7}  {}\n", k, r.calls, "-", num(r.shown as i64), "-", "-", basis(k)));
        } else {
            let pct = if r.raw > 0 { 100.0 * r.saved as f64 / r.raw as f64 } else { 0.0 };
            out.push_str(&format!("{:<8} {:>6} {:>12} {:>12} {:>12} {:>6.1}%  {}\n", k, r.calls, num(r.raw as i64), num(r.shown as i64), num(r.saved), pct, basis(k)));
        }
    }
    out
}
