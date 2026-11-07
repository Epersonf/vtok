//! The minifier: re-emits the leaves of a parse tree without comments and with the least whitespace that keeps the
//! tokens apart. Line breaks survive only where the grammar has them as tokens (Go and C/C++ directives do).
use crate::spec::{Mode, Spec};
use crate::tree::*;
use std::collections::{BTreeMap, HashSet};
use tree_sitter::Node;

pub struct Emit<'a> {
    src: &'a [u8],
    spec: &'a Spec,
    ren: &'a BTreeMap<String, String>,
    skeleton: bool,
    keep: bool,
    prev_kind: String,
    prev_row: usize,
    dir_rows: Vec<bool>,
    out: String,
    prev_end: usize,
    prev_text: String,
    skipped: Vec<(usize, usize)>,
    tops: HashSet<usize>,
}

/// Rows that belong to a preprocessor directive (a `#` line and its backslash continuations).
fn directive_rows(src: &[u8], on: bool) -> Vec<bool> {
    if !on { return vec![]; }
    let mut rows = vec![];
    let mut cont = false;
    for l in String::from_utf8_lossy(src).split(char::from(10)) {
        let is = cont || l.trim_start().starts_with('#');
        cont = is && l.trim_end().ends_with(char::from(92));
        rows.push(is);
    }
    rows
}

fn stringish(k: &str) -> bool { ["string", "template", "char_lit", "verbatim", "heredoc", "raw_"].iter().any(|s| k.contains(s)) }
const OPS: &str = "+-*/%&|^<>=!?:.~";

fn need_space(a: &str, b: &str) -> bool {
    let (Some(x), Some(y)) = (a.chars().last(), b.chars().next()) else { return false };
    if x == '\n' || y == '\n' { return false; }
    let quote = |c: char| matches!(c, '"' | '\'' | '`');
    (is_word_char(x) && is_word_char(y)) || (OPS.contains(x) && OPS.contains(y)) || (x.is_ascii_digit() && y == '.') || (x == '.' && y.is_ascii_digit()) || (quote(x) && quote(y))
}

fn css_keep(a: &str, b: &str) -> bool {
    let (Some(x), Some(y)) = (a.chars().last(), b.chars().next()) else { return false };
    (matches!(y, '.' | '#' | ':' | '[') && (is_word_char(x) || matches!(x, ']' | ')' | '*'))) || a == "+" || a == "-" || b == "+" || b == "-"
        || (is_word_char(x) && y == '(') || (x == ')' && is_word_char(y))
        || (matches!(y, '-' | '+') && (is_word_char(x) || x == ')'))
        || (matches!(x, ']' | ')') && (is_word_char(y) || matches!(y, '.' | '#' | ':' | '[' | '*' | '&')))
}

impl<'a> Emit<'a> {
    /// `tops` are the nodes that each start a new line (the top-level items, or the members of a chunk).
    pub fn new(src: &'a [u8], spec: &'a Spec, ren: &'a BTreeMap<String, String>, skeleton: bool, keep: bool, tops: &[Node]) -> Self {
        let tops = tops.iter().filter(|c| !is_comment(**c)).map(|c| c.start_byte()).collect();
        Emit { src, spec, ren, skeleton, keep, prev_kind: String::new(), prev_row: 0, dir_rows: directive_rows(src, spec.dir), out: String::new(), prev_end: 0, prev_text: String::new(), skipped: vec![], tops }
    }

    pub fn run(mut self, nodes: &[Node]) -> String {
        for n in nodes { self.visit(*n); }
        if !self.out.ends_with('\n') { self.out.push('\n'); }
        self.out
    }

    fn visit(&mut self, n: Node) {
        if is_comment(n) { self.skipped.push((n.start_byte(), n.end_byte())); return; }
        if self.skeleton && self.is_fn_body(n) { self.token(n, "{}".into()); self.prev_end = n.end_byte(); return; }
        if n.child_count() == 0 {
            if n.end_byte() == n.start_byte() { return; }
            let t = text(n, self.src);
            let t = if renamable(n, self.src) { self.ren.get(t).cloned().unwrap_or_else(|| t.to_string()) } else { t.to_string() };
            self.token(n, t);
            return;
        }
        for c in children(n) { self.visit(c); }
    }

    fn is_fn_body(&self, n: Node) -> bool {
        let Some(p) = n.parent() else { return false };
        (p.kind().contains("function") || p.kind().contains("method") || p.kind().contains("constructor"))
            && (n.kind().contains("block") || n.kind().contains("body")) && p.child_by_field_name("body").map_or(false, |b| b.id() == n.id())
    }

    /// Whitespace here is data (inside a string, a `<pre>`, raw script text): keep it as written.
    fn verbatim_context(&self, n: Node) -> bool {
        let mut p = n.parent();
        while let Some(a) = p {
            let k = a.kind();
            if stringish(k) && a.start_byte() < self.prev_end { return true; }
            if self.spec.mode == Mode::Html && matches!(k, "raw_text" | "script_element" | "style_element") && a.start_byte() < self.prev_end { return true; }
            if self.spec.mode == Mode::Html && k == "element" {
                let head = text(a, self.src).get(..10).unwrap_or("").to_ascii_lowercase();
                if (head.starts_with("<pre") || head.starts_with("<textarea")) && a.start_byte() < self.prev_end { return true; }
            }
            p = a.parent();
        }
        false
    }

    /// Source text between the previous token and `start`, with comments cut out.
    fn residual(&mut self, start: usize) -> String {
        let mut out = String::new();
        let mut at = self.prev_end;
        for &(s, e) in &self.skipped {
            if s >= at && s <= start { out.push_str(&String::from_utf8_lossy(&self.src[at..s])); at = e.min(start); }
        }
        if at < start { out.push_str(&String::from_utf8_lossy(&self.src[at..start])); }
        self.skipped.clear();
        out
    }

    /// Whether a single space must separate `a` and `b` (they were apart in the source).
    fn sep(&self, a: &str, b: &str) -> bool {
        if self.keep { return true; }
        match self.spec.mode {
            Mode::Html => !(matches!(a, "<" | "</" | "=") || matches!(b, ">" | "/>" | "=")),
            Mode::Css => need_space(a, b) || css_keep(a, b),
            Mode::Code => need_space(a, b),
        }
    }

    fn token(&mut self, n: Node, t: String) {
        let had_gap = n.start_byte() > self.prev_end;
        let residual = self.residual(n.start_byte().max(self.prev_end));
        let first = self.out.is_empty();
        let ws_only = residual.chars().all(char::is_whitespace);
        let nl = char::from(10);
        let adjacent_text = self.spec.mode == Mode::Html && n.kind() == "text" && self.prev_kind == "text";
        if !first && (adjacent_text || self.verbatim_context(n)) {
            self.out.push_str(&residual);
        } else if !first && !ws_only {
            // text the grammar keeps outside its leaves (digits before a unit, say): treat it as a token of its own
            let body = residual.trim();
            if self.sep(&self.prev_text, body) && residual.starts_with(char::is_whitespace) { self.out.push(' '); }
            self.out.push_str(body);
            if self.sep(body, &t) && residual.ends_with(char::is_whitespace) { self.out.push(' '); }
        } else if !first {
            let loose = matches!(self.prev_text.as_str(), "{" | "(" | "[" | ",") || t == "}";
            let dir = self.spec.dir && residual.contains(nl) && (self.dir_rows.get(self.prev_row).copied().unwrap_or(false) || self.dir_rows.get(n.start_position().row).copied().unwrap_or(false));
            if dir { if !self.out.ends_with(nl) { self.out.push(nl); } }
            else if self.spec.nl && residual.contains(nl) && !loose { if !self.out.ends_with(nl) { self.out.push(nl); } }
            else if self.tops.contains(&n.start_byte()) { if !self.out.ends_with(nl) { self.out.push(nl); } }
            else if had_gap && self.sep(&self.prev_text, &t) { self.out.push(' '); }
        }
        self.out.push_str(&t);
        self.prev_end = n.end_byte();
        self.prev_text = t;
        self.prev_kind = n.kind().to_string();
        self.prev_row = n.end_position().row;
    }
}
