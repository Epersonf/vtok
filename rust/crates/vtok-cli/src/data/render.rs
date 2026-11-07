//! The text of the view: a title, sample rows, one line per field, the odd records and the last row.
use super::flags::{self, Odd};
use super::records::{clip, Table};
use super::stats::{self, Col, Form};
use super::describe;

const HEAD: usize = 3;
const MAX_ODD: usize = 8;
const ID_NAMES: [&str; 11] = ["name", "id", "key", "uuid", "title", "slug", "email", "hostname", "host", "path", "label"];

pub fn view(t: &Table) -> Option<String> {
    let n = t.rows.len();
    let raw = t.rows.first()?.raw.is_some();
    let unit = if raw { "rows" } else { "records" };
    let cols = stats::analyze(t);
    let mut odd = flags::find(t, &cols);
    let total = odd.len();
    odd.truncate(MAX_ODD);
    odd.sort_by_key(|o| o.row);
    let label = if t.label.is_empty() { String::new() } else { format!("{} ", t.label) };
    let mut out = vec![format!("{}: {label}{n} {unit}, {} fields", t.kind, t.cols.len())];
    if let Some(a) = &t.around { out.push(format!("also: {a}")); }
    if let Some((at, text)) = &t.header { out.push(format!("{at}: {}", clip(text, 240))); }
    let mut shown: Vec<usize> = odd.iter().map(|o| o.row).collect();
    if raw {
        for r in 0..HEAD.min(n) { out.push(line(t, r)); }
        shown.extend(0..HEAD.min(n));
        shown.push(n - 1);
    } else {
        out.push(first(t));
        shown.push(0);
    }
    shown.sort_unstable();
    shown.dedup();
    if n > shown.len() { out.push(format!("… {} {unit} omitted …", n - shown.len())); }
    out.extend(describe::lines(t, &cols));
    out.extend(odd_lines(t, &cols, &odd, total, unit));
    if raw && !odd.iter().any(|o| o.row == n - 1) && n > HEAD { out.push(format!("last {}", line(t, n - 1))); }
    Some(out.join("\n") + "\n")
}

fn line(t: &Table, r: usize) -> String { format!("{}: {}", t.rows[r].at, clip(t.rows[r].raw.as_deref().unwrap_or(""), 200)) }

/// The first record of a JSON or YAML document as `field=value` pairs.
fn first(t: &Table) -> String {
    let pairs: Vec<String> = t.rows[0].cells.iter().enumerate().filter_map(|(c, v)| v.as_ref().map(|v| format!("{}={}", t.cols[c], clip(v, 28)))).collect();
    format!("first ({}): {}", t.rows[0].at, clip(&pairs.join(" "), 260))
}

fn odd_lines(t: &Table, cols: &[Col], odd: &[Odd], total: usize, unit: &str) -> Vec<String> {
    if odd.is_empty() { return vec!["outliers: none".to_string()]; }
    let id = (0..cols.len()).find(|&c| cols[c].form == Form::Unique && ID_NAMES.iter().any(|n| *n == t.cols[c].rsplit('.').next().unwrap_or("").to_lowercase()));
    let mut out = vec![format!("outliers ({total} {}):", if total == 1 { unit.trim_end_matches('s') } else { unit })];
    for o in odd {
        let mut why: Vec<String> = o.why.iter().take(4).cloned().collect();
        if o.why.len() > 4 { why.push(format!("+{} more", o.why.len() - 4)); }
        let why = why.join("; ");
        if t.rows[o.row].raw.is_some() {
            out.push(format!("{}  <- {why}", line(t, o.row)));
        } else {
            let who = id.and_then(|c| t.cell(o.row, c).map(|v| format!(" {}={}", t.cols[c], clip(v, 28)))).unwrap_or_default();
            out.push(format!("{}{who}: {why}", t.rows[o.row].at));
        }
    }
    if total > odd.len() { out.push(format!("  +{} more", total - odd.len())); }
    out
}
