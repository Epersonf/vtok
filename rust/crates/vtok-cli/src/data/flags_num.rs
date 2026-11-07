//! Odd cells in columns that have an order: numbers far outside the rest, text among numbers, breaks in a sorted column.
use super::flags::{rare_cap, Cell};
use super::records::{clip, Table};
use super::stats::{Col, Form};

/// Numbers more than three interquartile ranges beyond the quartiles, and text in a numeric column.
pub(super) fn numbers(t: &Table, c: usize, col: &Col, name: &str, out: &mut Vec<Cell>) {
    let Some(num) = &col.num else { return };
    let cap = rare_cap(col.present);
    let iqr = num.q3 - num.q1;
    let s = &num.sorted;
    let (lo, hi) = (num.q1 - 3.0 * iqr, num.q3 + 3.0 * iqr);
    let below = if iqr > 0.0 { s.iter().take_while(|x| x.0 < lo).count() } else { 0 };
    let above = if iqr > 0.0 { s.iter().rev().take_while(|x| x.0 > hi).count() } else { 0 };
    // more than a handful beyond the fences is a heavy tail, not a set of exceptions
    if (below > 0 || above > 0) && below + above <= (s.len() / 100).max(5) && below + above < s.len() {
        let (inner_lo, inner_hi) = (s[below].1, s[s.len() - above - 1].1);
        for &(x, r) in &s[..below] {
            out.push((r, format!("{name}={} (next lowest {})", t.cell(r, c).unwrap_or(""), t.cell(inner_lo, c).unwrap_or("")), 2.0 + ((lo - x) / iqr).log10().max(0.0)));
        }
        for &(x, r) in &s[s.len() - above..] {
            out.push((r, format!("{name}={} (next highest {})", t.cell(r, c).unwrap_or(""), t.cell(inner_hi, c).unwrap_or("")), 2.0 + ((x - hi) / iqr).log10().max(0.0)));
        }
    }
    let text: Vec<usize> = (0..t.rows.len()).filter(|&r| t.cell(r, c).map_or(false, |v| v.trim().parse::<f64>().map_or(true, |x| !x.is_finite()))).collect();
    if !text.is_empty() && text.len() <= cap {
        for r in text { out.push((r, format!("{name}={} (not a number; the others are)", clip(t.cell(r, c).unwrap_or(""), 40)), 2.5)); }
    }
}

/// A column that runs one way almost everywhere (timestamps, sequence numbers): the rows that go the other way. The
/// value is compared with the one before it, so a value that is too high shows up on the row after it.
pub(super) fn order(t: &Table, c: usize, col: &Col, name: &str, out: &mut Vec<Cell>) {
    if col.present < super::MIN_ROWS { return; }
    let numeric = col.form == Form::Num;
    let key = |v: &str| v.trim().parse::<f64>().unwrap_or(f64::NAN);
    let before = |a: &str, b: &str| if numeric { key(a) < key(b) } else { a < b };
    let vals: Vec<(usize, &str)> = (0..t.rows.len()).filter_map(|r| t.cell(r, c).map(|v| (r, v))).collect();
    let (mut rises, mut falls): (Vec<(usize, &str)>, Vec<(usize, &str)>) = (vec![], vec![]);
    for w in vals.windows(2) {
        if before(w[1].1, w[0].1) { falls.push((w[1].0, w[0].1)); } else if before(w[0].1, w[1].1) { rises.push((w[1].0, w[0].1)); }
    }
    let (steps, breaks, word) = if rises.len() >= falls.len() { (rises.len(), falls, "below") } else { (falls.len(), rises, "above") };
    if breaks.is_empty() || breaks.len() > rare_cap(col.present) || breaks.len() * 100 > steps { return; }
    for (r, prev) in breaks {
        out.push((r, format!("{name}={} (out of order: {word} {} before it)", clip(t.cell(r, c).unwrap_or(""), 40), clip(prev, 40)), 2.0));
    }
}
