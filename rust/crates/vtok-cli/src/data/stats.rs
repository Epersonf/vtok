//! Per-column statistics: what kind of field a column is and how its values are distributed.
use super::records::{shape, Table};
use std::collections::HashMap;

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum Form {
    /// Every value is the same.
    Constant,
    /// Numbers, at least eight different ones.
    Num,
    /// Every value differs (ids, timestamps, names).
    Unique,
    /// Few values, each used many times.
    Cat,
    /// Anything else: many values, some of them repeated.
    Many,
}

pub struct Num {
    pub q1: f64,
    pub median: f64,
    pub q3: f64,
    /// Every numeric value with the row it is in, smallest first.
    pub sorted: Vec<(f64, usize)>,
}

pub struct Col {
    pub form: Form,
    /// Rows that have a value.
    pub present: usize,
    pub distinct: usize,
    /// Values with their counts, most used first (every value of a `Cat`, a few of the others).
    pub top: Vec<(String, usize)>,
    /// Shapes of the values of `Unique` and `Many` columns with their counts, most used first.
    pub shapes: Vec<(String, usize)>,
    pub num: Option<Num>,
}

pub fn analyze(t: &Table) -> Vec<Col> { (0..t.cols.len()).map(|c| column(t, c)).collect() }

fn column(t: &Table, c: usize) -> Col {
    let mut counts: HashMap<&str, usize> = HashMap::new();
    let mut nums: Vec<(f64, usize)> = vec![];
    let mut present = 0;
    for (r, row) in t.rows.iter().enumerate() {
        let Some(v) = row.cells.get(c).and_then(|v| v.as_deref()) else { continue };
        present += 1;
        *counts.entry(v).or_insert(0) += 1;
        if let Ok(x) = v.trim().parse::<f64>() { if x.is_finite() { nums.push((x, r)); } }
    }
    let distinct = counts.len();
    let mut by_count: Vec<(&str, usize)> = counts.into_iter().collect();
    by_count.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
    let covered: usize = by_count.iter().take(8).map(|x| x.1).sum();
    nums.sort_by(|a, b| a.0.total_cmp(&b.0));
    let numeric = present > 0 && nums.len() * 20 >= present * 19 && distinct_nums(&nums) >= 8;
    let form = if distinct <= 1 { Form::Constant }
        else if numeric { Form::Num }
        else if distinct == present { Form::Unique }
        else if distinct <= 40 && distinct * 2 <= present && covered * 10 >= present * 9 { Form::Cat }
        else { Form::Many };
    let keep = if form == Form::Cat { by_count.len() } else { 3 };
    let top = by_count.iter().take(keep).map(|(v, n)| (v.to_string(), *n)).collect();
    let shapes = if matches!(form, Form::Unique | Form::Many) { shapes_of(t, c) } else { vec![] };
    let num = if form == Form::Num { Some(quartiles(nums)) } else { None };
    Col { form, present, distinct, top, shapes, num }
}

fn distinct_nums(sorted: &[(f64, usize)]) -> usize { 1 + sorted.windows(2).filter(|w| w[0].0 != w[1].0).count() }

fn quartiles(sorted: Vec<(f64, usize)>) -> Num {
    let at = |p: f64| sorted[((sorted.len() - 1) as f64 * p).round() as usize].0;
    Num { q1: at(0.25), median: at(0.5), q3: at(0.75), sorted }
}

/// Shape counts, most used first; more than 64 different shapes means the column has no shape worth reporting.
fn shapes_of(t: &Table, c: usize) -> Vec<(String, usize)> {
    let mut m: HashMap<String, usize> = HashMap::new();
    for row in &t.rows {
        if let Some(v) = row.cells.get(c).and_then(|v| v.as_deref()) {
            *m.entry(shape(v)).or_insert(0) += 1;
            if m.len() > 64 { break; }
        }
    }
    let mut v: Vec<(String, usize)> = m.into_iter().collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    v
}
