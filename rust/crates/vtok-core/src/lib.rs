//! Language-independent core: token counting, alias planning, errors.
pub mod aliasing;
pub mod lang;
pub mod language;
pub mod split;
pub mod tokens;

use std::fmt;

#[derive(Debug, Clone)]
pub struct VtokError {
    pub msg: String,
    pub line: Option<usize>,
}

impl VtokError {
    pub fn new(msg: impl Into<String>) -> Self {
        VtokError { msg: msg.into(), line: None }
    }
    pub fn at(msg: impl Into<String>, line: usize) -> Self {
        VtokError { msg: msg.into(), line: Some(line) }
    }
}

impl fmt::Display for VtokError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.line {
            Some(l) => write!(f, "line {}: {}", l, self.msg),
            None => write!(f, "{}", self.msg),
        }
    }
}

impl std::error::Error for VtokError {}

pub type Result<T> = std::result::Result<T, VtokError>;

/// Alias table text format: one `alias=original` per line, sorted by alias.
pub fn render_table(table: &std::collections::BTreeMap<String, String>) -> String {
    let mut rows: Vec<(&String, &String)> = table.iter().map(|(o, a)| (a, o)).collect();
    rows.sort();
    rows.iter().map(|(a, o)| format!("{a}={o}")).collect::<Vec<_>>().join("\n")
}

/// Parse a table file into `{alias: original}`.
pub fn parse_table(text: &str) -> std::collections::HashMap<String, String> {
    let mut t = std::collections::HashMap::new();
    for ln in text.lines() {
        if ln.trim().is_empty() {
            continue;
        }
        if let Some((a, o)) = ln.split_once('=') {
            t.insert(a.trim().to_string(), o.trim().to_string());
        }
    }
    t
}
