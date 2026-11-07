//! CSS views. Outline: one name per rule (its selectors) or @-rule (`@media (max-width: 600px)`). Skeleton: the
//! selector and @-rule tree with custom-property names and declaration counts instead of declarations
//! (`:root{--bg,--fg,+3}`, `.card{6}`, `@media (max-width:600px){.card{2}}`, `@keyframes spin{from,to}`).
//! Lean: the full view without vendor-prefixed duplicates of a declaration and with inline `data:` URLs elided.
use crate::emit::Emit;
use crate::spec::Spec;
use crate::tree::*;
use std::collections::{BTreeMap, HashSet};
use tree_sitter::Node;

/// Whitespace collapsed, no space after commas.
pub fn squeeze(s: &str) -> String { s.split_whitespace().collect::<Vec<_>>().join(" ").replace(", ", ",") }

fn block<'a>(n: Node<'a>) -> Option<Node<'a>> { children(n).into_iter().find(|c| matches!(c.kind(), "block" | "keyframe_block_list")) }

pub fn is_rule(n: Node) -> bool { n.kind() == "rule_set" || n.kind() == "at_rule" || n.kind().ends_with("_statement") }

/// A rule's or @-rule's head: its source text up to the block (or the whole statement without `;`).
pub fn header(n: Node, src: &[u8]) -> String {
    let end = block(n).map_or(n.end_byte(), |b| b.start_byte());
    squeeze(String::from_utf8_lossy(&src[n.start_byte()..end]).trim_end_matches(';'))
}

/// Rules nested in `n`'s block (inside @media, @supports, CSS nesting).
pub fn nested<'a>(n: Node<'a>) -> Vec<Node<'a>> { block(n).map(named_children).unwrap_or_default().into_iter().filter(|c| is_rule(*c)).collect() }

fn render(n: Node, src: &[u8], out: &mut String) {
    out.push_str(&header(n, src));
    let Some(b) = block(n) else { return };
    let items = named_children(b);
    if b.kind() == "keyframe_block_list" {
        let sel: Vec<String> = items.iter().filter(|k| k.kind() == "keyframe_block").filter_map(|k| k.named_child(0)).map(|s| squeeze(text(s, src))).collect();
        out.push_str(&format!("{{{}}}", sel.join(",")));
        return;
    }
    let decls: Vec<Node> = items.iter().copied().filter(|d| d.kind() == "declaration").collect();
    let custom: Vec<String> = decls.iter().filter_map(|d| d.named_child(0)).map(|p| text(p, src).to_string()).filter(|p| p.starts_with("--")).collect();
    let mut parts = vec![];
    let rest = decls.len() - custom.len();
    if !custom.is_empty() { parts.push(custom.join(",") + &if rest > 0 { format!(",+{rest}") } else { String::new() }); } else if rest > 0 { parts.push(rest.to_string()); }
    for r in items.iter().filter(|r| is_rule(**r)) { let mut s = String::new(); render(*r, src, &mut s); parts.push(s); }
    out.push_str(&format!("{{{}}}", parts.join(";")));
}

pub fn skeleton(src: &[u8], root: Node) -> String {
    let mut out = String::new();
    for n in named_children(root).into_iter().filter(|n| !is_comment(*n)) {
        render(n, src, &mut out);
        out.push('\n');
    }
    out
}

fn lean_marks(n: Node, src: &[u8], em: &mut Emit) {
    if n.kind() == "block" {
        let decls: Vec<Node> = named_children(n).into_iter().filter(|d| d.kind() == "declaration").collect();
        let prop = |d: &Node| d.named_child(0).map(|p| text(p, src).to_ascii_lowercase()).unwrap_or_default();
        let plain: HashSet<String> = decls.iter().map(prop).filter(|p| !p.starts_with('-')).collect();
        for d in &decls {
            let p = prop(d);
            let base = ["-webkit-", "-moz-", "-ms-", "-o-"].iter().find_map(|v| p.strip_prefix(v));
            if base.map_or(false, |b| plain.contains(b)) { em.skip.insert(d.id()); }
        }
    }
    if n.kind() == "arguments" && n.parent().and_then(|p| p.named_child(0)).map_or(false, |f| text(f, src) == "url") {
        let t = text(n, src);
        if t.len() > 80 && t.trim_start_matches(['(', '"', '\'', ' ']).starts_with("data:") { em.subst.insert(n.id(), format!("(data:…{} chars)", t.len())); }
    }
    for c in children(n) { lean_marks(c, src, em); }
}

pub fn lean(spec: &'static Spec, src: &[u8], root: Node) -> String {
    let none = BTreeMap::new();
    let tops = named_children(root);
    let mut em = Emit::new(src, spec, &none, false, false, &tops);
    lean_marks(root, src, &mut em);
    em.run(&children(root))
}
