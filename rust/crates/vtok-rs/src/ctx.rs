//! Emission context and joining tokens with the spaces they need.
use std::collections::{BTreeMap, HashMap};

pub(crate) struct Ctx<'a> {
    pub(crate) ren: &'a BTreeMap<String, String>,
    pub(crate) skeleton: bool,
    pub(crate) lean: bool,
    pub(crate) lits: &'a HashMap<usize, String>,
    pub(crate) src: &'a [u8],
}

const OPS2: &[&str] = &[
    "&&", "||", "<<", ">>", "<=", ">=", "==", "!=", "->", "=>", "::", "..", "+=", "-=", "*=", "/=", "%=", "^=", "&=", "|=", "<-", "/*", "//", "*/",
];
const OPS3: &[&str] = &["..=", "...", "<<=", ">>="];

pub(crate) fn isw(c: char) -> bool { c.is_alphanumeric() || c == '_' }

fn need_space(a: &str, b: &str) -> bool {
    let (Some(x), Some(y)) = (a.chars().last(), b.chars().next()) else { return false };
    if isw(x) && isw(y) { return true; }
    if !isw(x) && !isw(y) {
        let two = format!("{x}{y}");
        if OPS2.contains(&two.as_str()) { return true; }
        let ac: Vec<char> = a.chars().collect();
        let bc: Vec<char> = b.chars().collect();
        if ac.len() >= 2 && OPS3.contains(&format!("{}{x}{y}", ac[ac.len() - 2]).as_str()) { return true; }
        if bc.len() >= 2 && OPS3.contains(&format!("{x}{y}{}", bc[1]).as_str()) { return true; }
    }
    if !a.is_empty() && a.chars().all(|c| c.is_ascii_digit() || c == '_') && y == '.' && !b.starts_with("..") { return true; }
    if x == '.' && y.is_ascii_digit() && a.starts_with(|c: char| c.is_ascii_digit()) { return true; }
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
