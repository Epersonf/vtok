//! Syntax-tree helpers shared by every part of the Python plugin: parsing, the first syntax error, child access,
//! plain strings and docstrings.
use tree_sitter::{Node, Parser, Tree};

pub fn parse(src: &[u8]) -> Tree {
    let mut p = Parser::new();
    p.set_language(&tree_sitter_python::LANGUAGE.into()).expect("python grammar");
    p.parse(src, None).expect("parse")
}

pub fn first_error<'a>(n: Node<'a>) -> Option<Node<'a>> {
    if n.kind() == "ERROR" || n.is_missing() { return Some(n); }
    if !n.has_error() { return None; }
    for ch in children(n) {
        if let Some(e) = first_error(ch) { return Some(e); }
    }
    Some(n)
}

pub(crate) fn children<'a>(n: Node<'a>) -> Vec<Node<'a>> { let mut c = n.walk(); n.children(&mut c).collect() }
pub(crate) fn named_children<'a>(n: Node<'a>) -> Vec<Node<'a>> { let mut c = n.walk(); n.named_children(&mut c).collect() }
pub(crate) fn text<'a>(n: Node, src: &'a [u8]) -> &'a str { std::str::from_utf8(&src[n.start_byte()..n.end_byte()]).unwrap_or("") }
pub(crate) fn trivia(n: Node) -> bool { matches!(n.kind(), "comment" | "line_continuation") }

pub(crate) fn has_interpolation(n: Node) -> bool { children(n).iter().any(|c| c.kind() == "interpolation") }

/// `string` node that is a plain (non-f, non-bytes) str constant.
pub(crate) fn is_plain_str(n: Node, src: &[u8]) -> bool {
    if n.kind() != "string" || has_interpolation(n) { return false; }
    let start = children(n).into_iter().find(|c| c.kind() == "string_start");
    let pre = start.map(|s| text(s, src).to_lowercase()).unwrap_or_default();
    let prefix: String = pre.chars().take_while(|c| c.is_alphabetic()).collect();
    !prefix.contains('b') && !prefix.contains('f') && !prefix.contains('t')
}

pub(crate) fn is_docstring(stmt: Node, src: &[u8]) -> bool {
    if stmt.kind() != "expression_statement" { return false; }
    let kids: Vec<Node> = named_children(stmt).into_iter().filter(|c| !trivia(*c)).collect();
    if kids.len() != 1 { return false; }
    let k = kids[0];
    match k.kind() {
        "string" => is_plain_str(k, src),
        "concatenated_string" => named_children(k).iter().all(|s| s.kind() == "string" && is_plain_str(*s, src)),
        _ => false,
    }
}

pub(crate) fn body_statements<'a>(block: Node<'a>, strip_doc: bool, src: &[u8]) -> Vec<Node<'a>> {
    let mut v: Vec<Node> = named_children(block).into_iter().filter(|c| !trivia(*c)).collect();
    if strip_doc && v.first().map_or(false, |f| is_docstring(*f, src)) { v.remove(0); }
    v
}
