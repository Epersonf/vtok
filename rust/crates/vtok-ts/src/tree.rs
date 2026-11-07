//! Parsing (TypeScript, retried as TSX for JSX), the first syntax error and child access shared by every part of the plugin.
use tree_sitter::{Node, Parser, Tree};

pub fn parse(src: &[u8]) -> Tree {
    let mut p = Parser::new();
    p.set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()).expect("ts grammar");
    let t = p.parse(src, None).expect("parse");
    if !t.root_node().has_error() { return t; }
    // JSX / TSX: retry with the TSX grammar, keep it only if it parses cleanly.
    p.set_language(&tree_sitter_typescript::LANGUAGE_TSX.into()).expect("tsx grammar");
    match p.parse(src, None) {
        Some(x) if !x.root_node().has_error() => x,
        _ => t,
    }
}

pub fn first_error<'a>(n: Node<'a>) -> Option<Node<'a>> {
    if n.kind() == "ERROR" || n.is_missing() { return Some(n); }
    if !n.has_error() { return None; }
    let mut c = n.walk();
    for ch in n.children(&mut c) {
        if let Some(e) = first_error(ch) { return Some(e); }
    }
    Some(n)
}

pub(crate) fn text<'a>(n: Node, src: &'a [u8]) -> &'a str { std::str::from_utf8(&src[n.start_byte()..n.end_byte()]).unwrap_or("") }

pub(crate) fn children<'a>(n: Node<'a>) -> Vec<Node<'a>> { let mut c = n.walk(); n.children(&mut c).collect() }
pub(crate) fn named_children<'a>(n: Node<'a>) -> Vec<Node<'a>> { let mut c = n.walk(); n.named_children(&mut c).collect() }
