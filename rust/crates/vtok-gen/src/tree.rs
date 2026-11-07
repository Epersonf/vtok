//! Small tree helpers shared by the generic plugin: parsing, error lookup, identifier rules, canonical dump.
use crate::spec::Spec;
use tree_sitter::{Node, Parser, Tree};

pub fn parse(spec: &Spec, src: &[u8]) -> Tree {
    let mut p = Parser::new();
    p.set_language(&(spec.grammar)()).expect("grammar");
    p.parse(src, None).expect("parse")
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

pub fn text<'a>(n: Node, src: &'a [u8]) -> &'a str { std::str::from_utf8(&src[n.start_byte()..n.end_byte()]).unwrap_or("") }
pub fn children<'a>(n: Node<'a>) -> Vec<Node<'a>> { let mut c = n.walk(); n.children(&mut c).collect() }
pub fn named_children<'a>(n: Node<'a>) -> Vec<Node<'a>> { let mut c = n.walk(); n.named_children(&mut c).collect() }

pub fn is_comment(n: Node) -> bool { let k = n.kind(); k.contains("comment") || k == "marginalia" }
pub fn is_word_char(c: char) -> bool { c.is_alphanumeric() || c == '_' || c == '$' }

fn import_like(k: &str) -> bool { ["import", "using", "package", "include", "library", "part_"].iter().any(|s| k.contains(s)) }
fn ident_kind(k: &str) -> bool { k == "identifier" || k.ends_with("_identifier") }

/// A leaf the alias table may rename: identifier-shaped, outside imports, package clauses and the like.
pub fn renamable(n: Node, src: &[u8]) -> bool {
    let k = n.kind();
    if n.child_count() != 0 || !ident_kind(k) || k == "package_identifier" { return false; }
    let t = text(n, src);
    if !t.chars().next().map_or(false, |c| c.is_alphabetic() || c == '_') || !t.chars().all(is_word_char) { return false; }
    let mut p = n.parent();
    while let Some(a) = p { if import_like(a.kind()) { return false; } p = a.parent(); }
    true
}

/// Names that appear inside imports: never aliased anywhere, so imports stay readable and exact.
pub fn import_names(n: Node, src: &[u8], out: &mut std::collections::HashSet<String>, inside: bool) {
    let inside = inside || import_like(n.kind());
    if inside && n.child_count() == 0 && ident_kind(n.kind()) { out.insert(text(n, src).to_string()); }
    for c in children(n) { import_names(c, src, out, inside); }
}

/// Pre-order dump without comments; leaves carry their text. Two sources are equivalent when the dumps match.
pub fn canon(n: Node, src: &[u8], out: &mut Vec<String>) {
    if is_comment(n) { return; }
    if n.child_count() == 0 { out.push(format!("{}:{}", n.kind(), text(n, src))); return; }
    if children(n).iter().all(|c| is_comment(*c)) { out.push(format!("{}:", n.kind())); return; }
    out.push(format!("({}", n.kind()));
    for c in children(n) { canon(c, src, out); }
    out.push(")".into());
}

pub fn canon_of(spec: &Spec, s: &str) -> Option<Vec<String>> {
    let t = parse(spec, s.as_bytes());
    if first_error(t.root_node()).is_some() { return None; }
    let mut v = vec![];
    canon(t.root_node(), s.as_bytes(), &mut v);
    Some(v)
}
