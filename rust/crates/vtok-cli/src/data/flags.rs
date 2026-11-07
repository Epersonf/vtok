//! The cells that do not fit their column: a value seen once, a number far outside the rest, a field present in one
//! record only, a break in a sorted column. A record's score is the sum of how surprising its cells are.
use super::flags_cols::{duplicates, presence};
use super::flags_num::{numbers, order};
use super::records::{clip, shape, Table};
use super::stats::{Col, Form};
use std::collections::{BTreeMap, HashMap};

/// A record that stands out, with the reasons.
pub struct Odd { pub row: usize, pub why: Vec<String>, pub score: f64 }

/// (row, reason, score) of one odd cell.
pub(super) type Cell = (usize, String, f64);

/// Values seen at most this many times (and under 0.5% of the rows) are rare.
pub(super) fn rare_cap(n: usize) -> usize { (n / 200).clamp(1, 5) }

/// Odd records, most surprising first.
pub fn find(t: &Table, cols: &[Col]) -> Vec<Odd> {
    let mut cells: Vec<Cell> = vec![];
    // a record the reader could not split properly is odd first of all; its reason leads
    for (r, row) in t.rows.iter().enumerate() {
        if let Some(n) = &row.note { cells.push((r, n.clone(), 3.0)); }
    }
    for (c, col) in cols.iter().enumerate() {
        let name = t.cols[c].as_str();
        match col.form {
            Form::Cat => rare_values(t, c, col, name, &mut cells),
            Form::Num => numbers(t, c, col, name, &mut cells),
            Form::Unique | Form::Many => {
                rare_shapes(t, c, col, name, &mut cells);
                duplicates(t, c, col, name, &mut cells);
            }
            Form::Constant => {}
        }
        if matches!(col.form, Form::Num | Form::Unique | Form::Many) { order(t, c, col, name, &mut cells); }
        presence(t, c, col, name, &mut cells);
    }
    group(cells)
}

fn group(cells: Vec<Cell>) -> Vec<Odd> {
    let mut m: BTreeMap<usize, Odd> = BTreeMap::new();
    for (row, why, score) in cells {
        let o = m.entry(row).or_insert(Odd { row, why: vec![], score: 0.0 });
        o.why.push(why);
        o.score += score;
    }
    let mut v: Vec<Odd> = m.into_values().collect();
    v.sort_by(|a, b| b.score.total_cmp(&a.score).then(a.row.cmp(&b.row)));
    v
}

/// A column whose values are mostly few and common: the values used only a handful of times stand out.
fn rare_values(t: &Table, c: usize, col: &Col, name: &str, out: &mut Vec<Cell>) {
    let cap = rare_cap(col.present);
    let common: usize = col.top.iter().filter(|x| x.1 > cap).map(|x| x.1).sum();
    if common * 10 < col.present * 9 { return; }
    let rare: HashMap<&str, usize> = col.top.iter().filter(|x| x.1 <= cap).map(|x| (x.0.as_str(), x.1)).collect();
    let before = out.len();
    for (r, row) in t.rows.iter().enumerate() {
        let Some(v) = row.cells.get(c).and_then(|v| v.as_deref()) else { continue };
        if let Some(&k) = rare.get(v) {
            out.push((r, format!("{name}={} ({k} of {})", clip(v, 40), col.present), 1.0 + (col.present as f64 / k as f64).log10()));
            if out.len() - before >= 20 { break; }
        }
    }
}

/// Identifiers, timestamps, URLs: when nearly all have one shape, the ones with another shape stand out.
fn rare_shapes(t: &Table, c: usize, col: &Col, name: &str, out: &mut Vec<Cell>) {
    if col.shapes.len() < 2 || col.shapes.len() > 12 { return; }
    let cap = rare_cap(col.present);
    let common: usize = col.shapes.iter().filter(|x| x.1 > cap).map(|x| x.1).sum();
    if common * 10 < col.present * 9 { return; }
    let rare: HashMap<&str, usize> = col.shapes.iter().filter(|x| x.1 <= cap).map(|x| (x.0.as_str(), x.1)).collect();
    let before = out.len();
    for (r, row) in t.rows.iter().enumerate() {
        let Some(v) = row.cells.get(c).and_then(|v| v.as_deref()) else { continue };
        if let Some(&k) = rare.get(shape(v).as_str()) {
            out.push((r, format!("{name}={} (unlike the rest, {k} of {})", clip(v, 40), col.present), 1.0 + (col.present as f64 / k as f64).log10()));
            if out.len() - before >= 20 { break; }
        }
    }
}
