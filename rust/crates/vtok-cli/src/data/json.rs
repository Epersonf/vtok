//! JSON documents that are (or hold) a long list of similar objects, and newline-delimited JSON.
use super::records::{clip, Row, Table};
use serde_json::Value;
use std::collections::HashMap;

const MAX_COLS: usize = 120;
const MAX_RECORD: usize = 1 << 20;

/// The records of a whole-file JSON document.
pub fn read(lines: &[String]) -> Option<Table> {
    let text = lines.join("\n");
    let t = text.trim();
    if !(t.starts_with('{') || t.starts_with('[')) { return None; }
    from_value("json", &serde_json::from_str(t).ok()?)
}

/// The largest list of objects in `v` (or the largest object of objects) as a table, when its records share fields.
pub(super) fn from_value(kind: &'static str, v: &Value) -> Option<Table> {
    let (label, items, holder) = best(v, String::new(), 0)?;
    let mut t = table(kind, label, &items);
    if t.shared() < 3.min(t.cols.len()) { return None; }
    t.around = super::rest::around(v, holder);
    Some(t)
}

type Items<'a> = Vec<(String, &'a Value)>;
/// A label, the records, and the list or object that holds them.
type Found<'a> = (String, Items<'a>, &'a Value);

fn best<'a>(v: &'a Value, path: String, depth: usize) -> Option<Found<'a>> {
    let mut found: Option<Found<'a>> = None;
    let mut offer = |label: String, items: Items<'a>, holder: &'a Value| {
        if items.len() >= super::MIN_ROWS && found.as_ref().map_or(true, |f| items.len() > f.1.len()) { found = Some((label, items, holder)); }
    };
    match v {
        Value::Array(a) if !a.is_empty() && a.iter().all(Value::is_object) => offer(path.clone(), a.iter().enumerate().map(|(i, x)| (format!("{path}[{i}]"), x)).collect(), v),
        Value::Object(m) => {
            if m.len() >= super::MIN_ROWS && m.values().all(Value::is_object) { offer(path.clone(), m.iter().map(|(k, x)| (k.clone(), x)).collect(), v); }
            if depth < 3 {
                for (k, x) in m {
                    let p = if path.is_empty() { k.clone() } else { format!("{path}.{k}") };
                    if let Some((l, it, h)) = best(x, p, depth + 1) { offer(l, it, h); }
                }
            }
        }
        _ => {}
    }
    found
}

fn table(kind: &'static str, label: String, items: &Items) -> Table {
    let mut index: HashMap<String, usize> = HashMap::new();
    let mut cols: Vec<String> = vec![];
    let mut flat: Vec<Vec<(usize, String)>> = vec![];
    for (_, item) in items {
        let mut pairs = vec![];
        flatten("", item, 0, &mut pairs);
        let mut row = vec![];
        for (p, v) in pairs {
            let c = if let Some(&c) = index.get(&p) { c } else if cols.len() < MAX_COLS {
                index.insert(p.clone(), cols.len());
                cols.push(p);
                cols.len() - 1
            } else { continue };
            row.push((c, v));
        }
        flat.push(row);
    }
    let rows = items.iter().zip(flat).map(|((at, _), pairs)| {
        let mut cells = vec![None; cols.len()];
        for (c, v) in pairs { cells[c] = Some(v); }
        Row { at: at.clone(), raw: None, note: None, cells }
    }).collect();
    Table { kind, label, cols, rows, header: None, around: None }
}

/// Leaf values by dotted path; lists of objects keep their first three members (`a[0].b`), lists of scalars join.
fn flatten(prefix: &str, v: &Value, depth: usize, out: &mut Vec<(String, String)>) {
    match v {
        Value::Object(m) if depth < 6 => {
            for (k, x) in m { flatten(&if prefix.is_empty() { k.clone() } else { format!("{prefix}.{k}") }, x, depth + 1, out); }
        }
        Value::Array(a) if a.iter().any(|x| x.is_object() || x.is_array()) => {
            for (i, x) in a.iter().take(3).enumerate() { flatten(&format!("{prefix}[{i}]"), x, depth + 1, out); }
        }
        Value::Array(a) => {
            let items: Vec<String> = a.iter().map(|x| x.as_str().map_or_else(|| x.to_string(), String::from)).collect();
            out.push((prefix.to_string(), clip(&items.join(","), 80)));
        }
        Value::String(s) => out.push((prefix.to_string(), s.clone())),
        other => out.push((prefix.to_string(), other.to_string())),
    }
}

/// Newline-delimited JSON: one object per line, which stays available verbatim.
pub fn read_lines(lines: &[String]) -> Option<Table> {
    let mut items: Vec<(usize, &str, Value)> = vec![];
    let mut bad = 0;
    for (i, l) in lines.iter().enumerate() {
        let t = l.trim();
        if t.is_empty() { continue; }
        // a line of a megabyte or more is a whole document, not one record: not worth parsing twice
        match if t.len() <= MAX_RECORD { serde_json::from_str::<Value>(t).ok() } else { None } {
            Some(v) if v.is_object() => items.push((i, t, v)),
            _ => bad += 1,
        }
        if bad * 10 > items.len() + 10 { return None; }
    }
    if items.len() < super::MIN_ROWS || bad * 20 > items.len() { return None; }
    let pairs: Items = items.iter().map(|(i, _, v)| (format!("L{}", i + 1), v)).collect();
    let mut t = table("ndjson", String::new(), &pairs);
    for (row, (_, raw, _)) in t.rows.iter_mut().zip(&items) { row.raw = Some(raw.to_string()); }
    (t.shared() >= 3.min(t.cols.len())).then_some(t)
}
