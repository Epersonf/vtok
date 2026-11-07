//! The table every reader fills: one row per record, one column per field.

pub struct Table {
    /// First word of the view: csv, tsv, json, ndjson or yaml.
    pub kind: &'static str,
    /// What the records are called in the document (`items`, `services`); empty for CSV rows and top-level lists.
    pub label: String,
    pub cols: Vec<String>,
    pub rows: Vec<Row>,
    /// The header line of a CSV as (line label, text), kept verbatim.
    pub header: Option<(String, String)>,
    /// The other fields of the JSON or YAML document the records were found in (a total, a `next` link, a second list).
    pub around: Option<String>,
}

pub struct Row {
    /// Where the record is: `L1139` (a line of the file) or `items[88]` / `svc-31`.
    pub at: String,
    /// The record as the source printed it (CSV and NDJSON lines).
    pub raw: Option<String>,
    /// A problem the reader saw in the record itself (the wrong number of fields).
    pub note: Option<String>,
    /// One value per column; None where the record has no such field.
    pub cells: Vec<Option<String>>,
}

impl Table {
    pub fn cell(&self, r: usize, c: usize) -> Option<&str> { self.rows[r].cells.get(c).and_then(|v| v.as_deref()) }

    /// How many fields nine in ten records have. Objects that share none (a JSON Schema's properties, the steps of a
    /// workflow, a lockfile's entries) are the parts of a document, not the rows of one table.
    pub fn shared(&self) -> usize {
        let n = self.rows.len();
        (0..self.cols.len()).filter(|&c| self.rows.iter().filter(|r| r.cells[c].is_some()).count() * 10 >= n * 9).count()
    }
}

/// Digit runs become `#`, so `1.9.0-rc2` and `1.8.4` differ in shape while `152.38` and `5.01` do not.
pub fn shape(v: &str) -> String {
    let mut s = String::with_capacity(v.len());
    let mut digits = false;
    for ch in v.chars() {
        if ch.is_ascii_digit() {
            if !digits { s.push('#'); }
            digits = true;
        } else {
            s.push(ch);
            digits = false;
        }
    }
    s
}

/// At most `n` characters, with an ellipsis where it was cut.
pub fn clip(s: &str, n: usize) -> String {
    if s.chars().count() <= n { s.to_string() } else { format!("{}…", s.chars().take(n.saturating_sub(1)).collect::<String>()) }
}

/// A number without trailing noise: `152`, `152.38`.
pub fn num(x: f64) -> String {
    if x.fract() == 0.0 && x.abs() < 1e15 { return format!("{}", x as i64); }
    format!("{x:.4}").trim_end_matches('0').trim_end_matches('.').to_string()
}
