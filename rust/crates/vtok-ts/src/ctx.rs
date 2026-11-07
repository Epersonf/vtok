//! Emission context, joining tokens with the spaces they need, and JSX text normalization.
use std::collections::{BTreeMap, HashMap};

pub(crate) struct Ctx<'a> {
    pub(crate) ren: &'a BTreeMap<String, String>,
    pub(crate) skeleton: bool,
    pub(crate) lean: bool,
    pub(crate) lits: &'a HashMap<usize, String>,
    pub(crate) src: &'a [u8],
}

fn isw(c: char) -> bool { c.is_alphanumeric() || c == '_' || c == '$' }

fn need_space(a: &str, b: &str) -> bool {
    let (Some(x), Some(y)) = (a.chars().last(), b.chars().next()) else { return false };
    if isw(x) && isw(y) { return true; }
    if x == y && (x == '+' || x == '-') { return true; }
    if x == '/' && (y == '/' || y == '*') { return true; }
    if x == '?' && y == '.' { return true; }
    if x == '!' && y == '=' { return true; }
    if !a.is_empty() && a.chars().all(|c| c.is_ascii_digit()) && y == '.' { return true; }
    false
}

pub(crate) fn join(tokens: &[String]) -> String {
    let mut out = String::new();
    for (i, t) in tokens.iter().enumerate() {
        if i > 0 && need_space(&tokens[i - 1], t) { out.push(' '); }
        out.push_str(t);
    }
    out
}

/// JSX whitespace semantics: lines are trimmed (first keeps leading, last keeps trailing space),
/// empty lines dropped, the rest joined by one space. Text without a newline is unchanged.
pub(crate) fn jsx_norm(s: &str) -> String {
    if !s.contains('\n') { return s.to_string(); }
    let lines: Vec<&str> = s.split('\n').collect();
    let last = lines.len() - 1;
    let mut parts: Vec<&str> = vec![];
    for (i, l) in lines.iter().enumerate() {
        let mut l = *l;
        if i != 0 { l = l.trim_start_matches([' ', '\t', '\r']); }
        if i != last { l = l.trim_end_matches([' ', '\t', '\r']); }
        if !l.is_empty() { parts.push(l); }
    }
    parts.join(" ")
}
