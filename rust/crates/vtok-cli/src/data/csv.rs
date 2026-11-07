//! Delimited text: CSV, TSV and the like. The first line is the header when it names the columns.
use super::records::{Row, Table};
use std::collections::HashMap;

/// Split one line on `d`; double quotes group, and `""` inside them is a quote.
pub fn split(line: &str, d: char) -> Vec<String> {
    let (mut out, mut cur, mut quoted) = (vec![], String::new(), false);
    let mut it = line.chars().peekable();
    while let Some(ch) = it.next() {
        match ch {
            '"' if quoted && it.peek() == Some(&'"') => {
                cur.push('"');
                it.next();
            }
            '"' => quoted = !quoted,
            c if c == d && !quoted => out.push(std::mem::take(&mut cur)),
            c => cur.push(c),
        }
    }
    out.push(cur);
    out
}

/// The delimiter and field count that at least 90% of the first lines agree on. Guessing from content only tries
/// comma and tab; a `.csv`/`.tsv` name also allows `;` and `|`.
fn delimiter(lines: &[&str], hinted: bool) -> Option<(char, usize)> {
    let sample: Vec<&str> = lines.iter().copied().take(60).collect();
    let cands: &[char] = if hinted { &[',', '\t', ';', '|'] } else { &[',', '\t'] };
    let min_fields = if hinted { 2 } else { 3 };
    let mut best: Option<(char, usize)> = None;
    for &d in cands {
        let mut freq: HashMap<usize, usize> = HashMap::new();
        for l in &sample { *freq.entry(split(l, d).len()).or_insert(0) += 1; }
        let (&m, &k) = freq.iter().max_by_key(|x| (*x.1, *x.0))?;
        if m >= min_fields && k * 10 >= sample.len() * 9 && best.map_or(true, |b| m > b.1) { best = Some((d, m)); }
    }
    best
}

fn is_header(first: &[String]) -> bool {
    let mut names: Vec<&str> = first.iter().map(|c| c.trim()).collect();
    names.sort_unstable();
    names.dedup();
    names.len() == first.len() && first.iter().all(|c| !c.trim().is_empty() && c.trim().parse::<f64>().is_err())
}

/// `hinted`: the command named a CSV-like file, so a wider choice of delimiters is believable.
pub fn read(lines: &[String], hinted: bool) -> Option<Table> {
    let body: Vec<(usize, &str)> = lines.iter().enumerate().filter(|(_, l)| !l.trim().is_empty()).map(|(i, l)| (i, l.as_str())).collect();
    if body.len() <= super::MIN_ROWS { return None; }
    let texts: Vec<&str> = body.iter().map(|x| x.1).collect();
    let (d, m) = delimiter(&texts, hinted)?;
    let first = split(texts[0], d);
    let header = is_header(&first);
    let cols: Vec<String> = if header { first.iter().map(|c| c.trim().to_string()).collect() } else { (1..=m).map(|i| format!("col{i}")).collect() };
    let mut rows = vec![];
    let mut ragged = 0;
    for &(i, l) in &body[usize::from(header)..] {
        let mut cells = split(l, d);
        let note = (cells.len() != m).then(|| format!("{} fields, expected {m}", cells.len()));
        ragged += usize::from(note.is_some());
        cells.resize(m, String::new());
        let cells = cells.into_iter().map(|c| if c.trim().is_empty() { None } else { Some(c) }).collect();
        rows.push(Row { at: format!("L{}", i + 1), raw: Some(l.to_string()), note, cells });
    }
    if ragged * 50 > rows.len() { return None; }
    let header = header.then(|| (format!("L{}", body[0].0 + 1), texts[0].to_string()));
    Some(Table { kind: if d == '\t' { "tsv" } else { "csv" }, label: String::new(), cols, rows, header, around: None })
}
