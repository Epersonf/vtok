//! Which tokens must stay apart when the minifier joins them, per grammar family.
use crate::spec::Mode;
use crate::tree::is_word_char;
use tree_sitter::Node;

/// Rows that belong to a preprocessor directive (a `#` line and its backslash continuations).
pub fn directive_rows(src: &[u8], on: bool) -> Vec<bool> {
    if !on { return vec![]; }
    let mut rows = vec![];
    let mut cont = false;
    for l in String::from_utf8_lossy(src).split(char::from(10)) {
        let is = cont || l.trim_start_matches(|c: char| c.is_whitespace() || c == '\u{feff}').starts_with('#');
        cont = is && l.trim_end().ends_with(char::from(92));
        rows.push(is);
    }
    rows
}

pub fn stringish(k: &str) -> bool { ["string", "template", "char_lit", "verbatim", "heredoc", "raw_"].iter().any(|s| k.contains(s)) }
const OPS: &str = "+-*/%&|^<>=!?:.~";

pub fn need_space(a: &str, b: &str) -> bool {
    let (Some(x), Some(y)) = (a.chars().last(), b.chars().next()) else { return false };
    if x == '\n' || y == '\n' { return false; }
    let quote = |c: char| matches!(c, '"' | '\'' | '`');
    (is_word_char(x) && is_word_char(y)) || (OPS.contains(x) && OPS.contains(y)) || (x.is_ascii_digit() && y == '.') || (x == '.' && y.is_ascii_digit()) || (quote(x) && quote(y))
}

pub fn css_keep(a: &str, b: &str) -> bool {
    let (Some(x), Some(y)) = (a.chars().last(), b.chars().next()) else { return false };
    (matches!(y, '.' | '#' | ':' | '[') && (is_word_char(x) || matches!(x, ']' | ')' | '*'))) || a == "+" || a == "-" || b == "+" || b == "-"
        || (is_word_char(x) && y == '(') || (x == ')' && is_word_char(y))
        || (matches!(y, '-' | '+') && (is_word_char(x) || x == ')'))
        || (matches!(x, ']' | ')') && (is_word_char(y) || matches!(y, '.' | '#' | ':' | '[' | '*' | '&')))
}

/// The body of a function, method or constructor (dropped by the skeleton view).
pub fn is_fn_body(n: Node) -> bool {
    let Some(p) = n.parent() else { return false };
    (p.kind().contains("function") || p.kind().contains("method") || p.kind().contains("constructor"))
        && (n.kind().contains("block") || n.kind().contains("body") || n.kind() == "compound_statement") && p.child_by_field_name("body").map_or(false, |b| b.id() == n.id())
}

/// Whether a single space must separate `a` and `b` (they were apart in the source).
pub fn sep(mode: Mode, a: &str, b: &str) -> bool {
    match mode {
        Mode::Html => !(matches!(a, "<" | "</" | "=") || matches!(b, ">" | "/>" | "=")),
        Mode::Css => need_space(a, b) || css_keep(a, b),
        Mode::Code => need_space(a, b),
    }
}
