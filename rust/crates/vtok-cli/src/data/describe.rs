//! One line per field: what the values of a column look like.
use super::records::{clip, num, Table};
use super::stats::{Col, Form};

/// The field lines together stay under this many characters (about 600 tokens); wider tables end in "+N more fields".
const MAX_FIELD_CHARS: usize = 2400;
/// Fields that fewer than half of the records have: only this many get a line (the most common first), so the keys of a
/// map written out as columns (dependency names, tool names) do not crowd out the fields that describe the records.
const MAX_SPARSE: usize = 6;

pub fn lines(t: &Table, cols: &[Col]) -> Vec<String> {
    let n = t.rows.len();
    let mut out = vec!["fields:".to_string()];
    let same: Vec<String> = (0..cols.len())
        .filter(|&i| cols[i].form == Form::Constant && cols[i].present == n)
        .map(|i| format!("{}={}", t.cols[i], clip(&cols[i].top[0].0, 24)))
        .collect();
    if !same.is_empty() { out.push(format!("  same in all: {}", clip(&same.join(" "), 360))); }
    let rest: Vec<usize> = (0..cols.len()).filter(|&i| cols[i].present > 0 && !(cols[i].form == Form::Constant && cols[i].present == n)).collect();
    let (dense, mut sparse): (Vec<usize>, Vec<usize>) = rest.iter().partition(|&&i| cols[i].present * 2 >= n);
    sparse.sort_by_key(|&i| std::cmp::Reverse(cols[i].present));
    let order: Vec<usize> = dense.iter().copied().chain(sparse.iter().copied().take(MAX_SPARSE)).collect();
    let (mut used, mut shown) = (0, 0);
    for &i in &order {
        let part = if cols[i].present < n { format!(" (in {} of {n})", cols[i].present) } else { String::new() };
        let line = format!("  {}: {}{part}", t.cols[i], describe(t, i, &cols[i]));
        used += line.len();
        if used > MAX_FIELD_CHARS { break; }
        out.push(line);
        shown += 1;
    }
    if shown < rest.len() { out.push(format!("  +{} more fields", rest.len() - shown)); }
    out
}

fn describe(t: &Table, c: usize, col: &Col) -> String {
    match (col.form, &col.num) {
        (Form::Constant, _) => format!("always {}", clip(&col.top[0].0, 40)),
        (Form::Num, Some(nm)) => {
            let at = |r: usize| clip(t.cell(r, c).unwrap_or(""), 16);
            format!("{} … {}, median {}", at(nm.sorted[0].1), at(nm.sorted[nm.sorted.len() - 1].1), num(nm.median))
        }
        (Form::Unique, _) => {
            let vals: Vec<&str> = (0..t.rows.len()).filter_map(|r| t.cell(r, c)).collect();
            format!("unique, {} … {}", clip(vals[0], 28), clip(vals[vals.len() - 1], 28))
        }
        (Form::Cat, _) => {
            let mut s = col.top.iter().take(8).map(|(v, k)| format!("{} {k}", clip(v, 24))).collect::<Vec<_>>().join(", ");
            if col.distinct > 8 { s.push_str(&format!(", +{} more values", col.distinct - 8)); }
            s
        }
        _ => format!("{} distinct, e.g. {}", col.distinct, col.top.iter().take(2).map(|x| clip(&x.0, 28)).collect::<Vec<_>>().join(", ")),
    }
}
