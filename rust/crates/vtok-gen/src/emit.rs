//! The minifier: re-emits the leaves of a parse tree without comments and with the least whitespace that keeps the
//! tokens apart. Line breaks survive only where the grammar has them as tokens (Go and C/C++ directives do).
//! Exact-copy paths (unparsed regions, protected strings) are in `verbatim.rs`.
use crate::spec::{Mode, Spec};
use crate::space::*;
use crate::tree::*;
use std::collections::{BTreeMap, HashMap, HashSet};
use tree_sitter::Node;

pub struct Emit<'a> {
    pub(crate) src: &'a [u8],
    pub(crate) spec: &'a Spec,
    ren: &'a BTreeMap<String, String>,
    skeleton: bool,
    keep: bool,
    /// Unparsed regions are copied whole even when they contain parsed structure (the fallback attempt).
    pub coarse: bool,
    /// Spans copied exactly: string literals by the lexer's reading (see `lex.rs`), sorted.
    pub protect: Vec<(usize, usize)>,
    /// Lean views: nodes left out (by id) and nodes replaced by a marker text (by id).
    pub skip: HashSet<usize>,
    pub subst: HashMap<usize, String>,
    pub(crate) prev_kind: String,
    prev_row: usize,
    pub(crate) prev_end: usize,
    prev_text: String,
    dir_rows: Vec<bool>,
    pub(crate) out: String,
    pub(crate) skipped: Vec<(usize, usize)>,
    tops: HashSet<usize>,
    /// The previous token was an unparsed region: the next gap is copied exactly.
    pub(crate) raw_next: bool,
    /// Where the first emitted token starts and whether it is an unparsed region.
    pub(crate) first: Option<(usize, bool)>,
}

impl<'a> Emit<'a> {
    /// `tops` are the nodes that each start a new line (the top-level items, or the members of a chunk).
    pub fn new(src: &'a [u8], spec: &'a Spec, ren: &'a BTreeMap<String, String>, skeleton: bool, keep: bool, tops: &[Node]) -> Self {
        let tops = tops.iter().filter(|c| !is_comment(**c)).map(|c| c.start_byte()).collect();
        Emit { src, spec, ren, skeleton, keep, coarse: false, protect: vec![], skip: HashSet::new(), subst: HashMap::new(), prev_kind: String::new(), prev_row: 0, dir_rows: directive_rows(src, spec.dir), out: String::new(), prev_end: 0, prev_text: String::new(), skipped: vec![], tops, raw_next: false, first: None }
    }

    pub fn run(self, nodes: &[Node]) -> String {
        let mut out = self.run_seg(nodes).0;
        if !out.ends_with('\n') { out.push('\n'); }
        out
    }

    /// The text, where its first token starts in the source, whether that token is an unparsed region and whether
    /// the text ends with one (joins next to an unparsed region must copy the source exactly).
    pub fn run_seg(mut self, nodes: &[Node]) -> (String, usize, bool, bool) {
        for n in nodes { self.visit(*n); }
        let (at, raw) = self.first.unwrap_or((0, false));
        let last = self.prev_kind == "ERROR";
        (self.out, at, raw, last)
    }

    fn visit(&mut self, n: Node) {
        if self.skip.contains(&n.id()) { self.skipped.push((n.start_byte(), n.end_byte())); return; }
        if let Some(t) = self.subst.get(&n.id()).cloned() { self.token(n, t); self.prev_end = n.end_byte(); return; }
        // kept where dropping it would change the tree: a comment inside a #define ends the macro text there for the
        // grammar, and one between two HTML text nodes keeps them apart; "comments" inside a lexer string are text
        let prot = !self.protect.is_empty() && self.protected(n.start_byte());
        let in_macro = self.spec.dir && n.parent().map_or(false, |p| matches!(p.kind(), "preproc_def" | "preproc_function_def"));
        let in_text = self.spec.mode == Mode::Html && self.prev_kind == "text" && n.next_sibling().map_or(false, |s| s.kind() == "text");
        if is_comment(n) && !in_macro && !in_text && !prot { self.skipped.push((n.start_byte(), n.end_byte())); return; }
        if is_comment(n) { return self.token(n, text(n, self.src).to_string()); }
        let orphan = n.child_count() == 0 && n.parent().map_or(false, |p| p.is_error()); // a stray token in an unparsed region
        if (n.is_error() && (self.coarse || is_opaque(n))) || orphan { self.raw(n); return; }
        if self.skeleton && is_fn_body(n) { self.token(n, "{}".into()); self.prev_end = n.end_byte(); return; }
        if n.child_count() == 0 {
            if n.end_byte() == n.start_byte() { return; }
            let t = text(n, self.src);
            if prot { return self.exact(n, t.to_string()); }
            let t = if renamable(n, self.src) { self.ren.get(t).cloned().unwrap_or_else(|| t.to_string()) } else { t.to_string() };
            self.token(n, t);
            return;
        }
        for c in children(n) { self.visit(c); }
    }

    fn sep(&self, a: &str, b: &str) -> bool { self.keep || sep(self.spec.mode, a, b) }

    fn token(&mut self, n: Node, t: String) {
        if std::mem::take(&mut self.raw_next) || (!self.protect.is_empty() && self.protected(n.start_byte())) { return self.exact(n, t); }
        let had_gap = n.start_byte() > self.prev_end;
        let residual = self.residual(n.start_byte().max(self.prev_end));
        let first = self.out.is_empty();
        let ws_only = residual.chars().all(char::is_whitespace);
        let nl = char::from(10);
        let adjacent_text = self.spec.mode == Mode::Html && n.kind() == "text" && self.prev_kind == "text";
        let continuation = !ws_only && residual.trim().chars().all(|c| c == '\\' || c.is_whitespace()); // a backslash line splice: keep exactly
        if !first && (adjacent_text || continuation || self.verbatim_context(n)) {
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
        self.put(n, t)
    }

    pub(crate) fn put(&mut self, n: Node, t: String) {
        self.first.get_or_insert((n.start_byte(), false));
        self.out.push_str(&t);
        self.prev_end = n.end_byte();
        self.prev_text = t;
        self.prev_kind = n.kind().to_string();
        self.prev_row = n.end_position().row;
    }
}
