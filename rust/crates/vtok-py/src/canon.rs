//! Canonical dump of a syntax tree, used to check that two sources mean the same (comments and docstrings aside).
use crate::tree::{children, is_docstring, text, trivia};
use tree_sitter::Node;

pub(crate) fn canon(n: Node, src: &[u8], strip_docs: bool, out: &mut Vec<String>) {
    let t = n.kind();
    if trivia(n) { return; }
    if !n.is_named() {
        if t != ";" { out.push(t.to_string()); }
        return;
    }
    if t == "string" { out.push(format!("string:{}", text(n, src))); return; }
    if n.child_count() == 0 && t != "module" { out.push(format!("{t}:{}", text(n, src))); return; }
    out.push(format!("({t}"));
    let body_owner = strip_docs && matches!(t, "module" | "block") && n.parent().map_or(true, |p| matches!(p.kind(), "function_definition" | "class_definition") || t == "module");
    let mut first_stmt = body_owner;
    let mut emitted = false;
    for c in children(n) {
        if trivia(c) { continue; }
        if first_stmt && c.is_named() {
            first_stmt = false;
            if is_docstring(c, src) { continue; }
        }
        if c.is_named() || c.kind() != ";" { emitted |= c.is_named(); }
        canon(c, src, strip_docs, out);
    }
    if body_owner && t == "block" && !emitted {
        out.extend(["(pass_statement".to_string(), "pass".to_string(), ")".to_string()]);
    }
    out.push(")".into());
}
