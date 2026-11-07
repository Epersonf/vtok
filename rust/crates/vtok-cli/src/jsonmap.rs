//! Minimal JSON (string -> string) so the CLI needs no extra dependency.
use std::collections::{BTreeMap, HashMap};

pub(crate) fn json_map(m: &BTreeMap<String, String>) -> String {
    let esc = |s: &str| s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n").replace('\r', "\\r").replace('\t', "\\t");
    format!("{{{}}}", m.iter().map(|(k, v)| format!("\"{}\":\"{}\"", esc(k), esc(v))).collect::<Vec<_>>().join(","))
}

pub(crate) fn parse_json_map(s: &str) -> HashMap<String, String> {
    let b: Vec<char> = s.chars().collect();
    let mut i = 0;
    let mut out = HashMap::new();
    let read_str = |i: &mut usize| -> Option<String> {
        while *i < b.len() && b[*i] != '"' { if b[*i] == '}' { return None; } *i += 1; }
        if *i >= b.len() { return None; }
        *i += 1;
        let mut r = String::new();
        while *i < b.len() && b[*i] != '"' {
            if b[*i] == '\\' {
                *i += 1;
                match b[*i] {
                    'n' => r.push('\n'), 'r' => r.push('\r'), 't' => r.push('\t'),
                    'u' => { let h: String = b[*i + 1..*i + 5].iter().collect(); r.push(char::from_u32(u32::from_str_radix(&h, 16).unwrap_or(63)).unwrap_or('?')); *i += 4; }
                    c => r.push(c),
                }
            } else { r.push(b[*i]); }
            *i += 1;
        }
        *i += 1;
        Some(r)
    };
    while let Some(k) = read_str(&mut i) {
        match read_str(&mut i) { Some(v) => { out.insert(k, v); } None => break }
    }
    out
}
