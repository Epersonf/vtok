//! Odd cells found by looking at a whole column: fields that few records have or lack, repeated ids.
use super::flags::{rare_cap, Cell};
use super::records::{clip, Table};
use super::stats::{Col, Form};
use std::collections::HashMap;

/// A field that only a few records have, or that only a few records lack.
pub(super) fn presence(t: &Table, c: usize, col: &Col, name: &str, out: &mut Vec<Cell>) {
    let n = t.rows.len();
    if n < super::MIN_ROWS || col.present == 0 { return; }
    let cap = rare_cap(n);
    let absent = n - col.present;
    if absent > 0 && absent <= cap {
        // a short row (reader's note) lacks its last fields for that reason alone
        for r in (0..n).filter(|&r| t.cell(r, c).is_none() && t.rows[r].note.is_none()) { out.push((r, format!("no {name} ({} of {n} have it)", col.present), 2.0)); }
    } else if col.present <= cap {
        for r in (0..n).filter(|&r| t.cell(r, c).is_some()) {
            out.push((r, format!("{name}={} (only {} of {n} have it)", clip(t.cell(r, c).unwrap_or(""), 40), col.present), 2.5));
        }
    }
}

/// A column where 99% of the values are different (ids, keys): the values that appear twice.
pub(super) fn duplicates(t: &Table, c: usize, col: &Col, name: &str, out: &mut Vec<Cell>) {
    if col.form != Form::Many || col.distinct == col.present || col.distinct * 100 < col.present * 99 { return; }
    let twice: HashMap<&str, usize> = col.top.iter().filter(|x| x.1 > 1).map(|x| (x.0.as_str(), x.1)).collect();
    for (r, row) in t.rows.iter().enumerate() {
        let Some(v) = row.cells.get(c).and_then(|v| v.as_deref()) else { continue };
        if let Some(&k) = twice.get(v) { out.push((r, format!("{name}={} (appears {k} times in a column that is otherwise unique)", clip(v, 40)), 2.0)); }
    }
}
