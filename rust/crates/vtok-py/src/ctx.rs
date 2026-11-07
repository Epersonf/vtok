//! Emission context and the small helpers every emitter uses: the context, one output line, token joining, renames
//! and hidden literals.
use crate::tree::{children, text};
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, HashSet};
use tree_sitter::Node;

pub(crate) struct Ctx<'a> {
    pub(crate) ren: &'a BTreeMap<String, String>,
    pub(crate) lean: bool,
    pub(crate) skeleton: bool,
    pub(crate) hide: usize,
    pub(crate) existing: &'a HashSet<String>,
    pub(crate) lits: RefCell<(BTreeMap<String, String>, HashMap<String, String>, usize)>, // placeholder->text, text->placeholder, counter
    pub(crate) src: &'a [u8],
}

pub(crate) struct Line { pub(crate) indent: usize, pub(crate) text: String, pub(crate) simple: bool }

fn isword_end(s: &str) -> bool { s.chars().last().map_or(false, |c| c.is_alphanumeric() || c == '_' || c == '\'' || c == '"') }
fn isword_start(s: &str) -> bool { s.chars().next().map_or(false, |c| c.is_alphanumeric() || c == '_' || c == '\'' || c == '"') }

pub(crate) fn join(tokens: &[String]) -> String {
    let mut out = String::new();
    for (i, t) in tokens.iter().enumerate() {
        if i > 0 {
            let p = &tokens[i - 1];
            let numdot = !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()) && t == ".";
            if (isword_end(p) && isword_start(t)) || numdot { out.push(' '); }
        }
        out.push_str(t);
    }
    out
}

pub(crate) fn rename_of(s: &str, ctx: &Ctx) -> String { ctx.ren.get(s).cloned().unwrap_or_else(|| s.to_string()) }

/// Source text of `n` with identifier descendants renamed (used for f-strings).
pub(crate) fn text_renamed(n: Node, ctx: &Ctx) -> String {
    let mut spans: Vec<(usize, usize, String)> = vec![];
    let mut st = vec![n];
    while let Some(x) = st.pop() {
        for c in children(x) { st.push(c); }
        if x.kind() == "identifier" {
            let s = text(x, ctx.src);
            if let Some(r) = ctx.ren.get(s) { spans.push((x.start_byte() - n.start_byte(), x.end_byte() - n.start_byte(), r.clone())); }
        }
    }
    spans.sort_by(|a, b| b.0.cmp(&a.0));
    let mut b = text(n, ctx.src).as_bytes().to_vec();
    for (a, e, r) in spans { b.splice(a..e, r.into_bytes()); }
    String::from_utf8_lossy(&b).into_owned()
}

pub(crate) fn literal_placeholder(s: &str, ctx: &Ctx) -> String {
    let mut g = ctx.lits.borrow_mut();
    if let Some(p) = g.1.get(s) { return p.clone(); }
    let mut k = g.2;
    loop {
        k += 1;
        let ph = format!("'${k}'");
        if !ctx.existing.contains(&ph) { g.2 = k; g.0.insert(ph.clone(), s.to_string()); g.1.insert(s.to_string(), ph.clone()); return ph; }
    }
}

pub(crate) fn is_field(parent: Node, child: Node, name: &str) -> bool {
    parent.child_by_field_name(name).map_or(false, |f| f.id() == child.id())
}
