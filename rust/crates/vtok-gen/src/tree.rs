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

/// A region the grammar could not parse: it is carried as exact source text (an opaque verbatim chunk), unless it
/// holds well-formed items (HTML elements after a stray tag, say), which are then minified one by one.
pub fn is_opaque(n: Node) -> bool {
    n.is_error() && !children(n).iter().any(|c| c.is_named() && c.child_count() > 0 && !c.has_error() && !is_comment(*c))
}

pub fn text<'a>(n: Node, src: &'a [u8]) -> &'a str { std::str::from_utf8(&src[n.start_byte()..n.end_byte()]).unwrap_or("") }
pub fn children<'a>(n: Node<'a>) -> Vec<Node<'a>> { let mut c = n.walk(); n.children(&mut c).collect() }
pub fn named_children<'a>(n: Node<'a>) -> Vec<Node<'a>> { let mut c = n.walk(); n.named_children(&mut c).collect() }

/// Comment nodes (not SQL's `COMMENT ON` statement nor its `COMMENT` keyword, which are code).
pub fn is_comment(n: Node) -> bool { let k = n.kind(); k == "comment" || (k.ends_with("_comment") && !k.starts_with("keyword")) || k == "marginalia" }
pub fn is_word_char(c: char) -> bool { c.is_alphanumeric() || c == '_' || c == '$' }

fn import_like(k: &str) -> bool { ["import", "using", "package", "include", "library", "part_"].iter().any(|s| k.contains(s)) }
fn ident_kind(k: &str) -> bool { k == "identifier" || k.ends_with("_identifier") }

/// A leaf the alias table may rename: identifier-shaped, outside imports, package clauses, unparsed regions and the like.
pub fn renamable(n: Node, src: &[u8]) -> bool {
    let k = n.kind();
    if n.child_count() != 0 || !ident_kind(k) || k == "package_identifier" { return false; }
    let t = text(n, src);
    if !t.chars().next().map_or(false, |c| c.is_alphabetic() || c == '_') || !t.chars().all(is_word_char) { return false; }
    let mut p = n.parent();
    while let Some(a) = p { if import_like(a.kind()) || is_opaque(a) { return false; } p = a.parent(); }
    true
}

/// Names that appear inside imports: never aliased anywhere, so imports stay readable and exact.
pub fn import_names(n: Node, src: &[u8], out: &mut std::collections::HashSet<String>, inside: bool) {
    let inside = inside || import_like(n.kind());
    if inside && n.child_count() == 0 && ident_kind(n.kind()) { out.insert(text(n, src).to_string()); }
    for c in children(n) { import_names(c, src, out, inside); }
}

/// Non-blank source text between the children of `n` that no leaf covers (CSS keeps the digits of `1px` there).
fn gaps(src: &[u8], out: &mut Vec<String>, at: &mut usize, upto: usize, flat: bool) {
    if *at < upto {
        let g = String::from_utf8_lossy(&src[*at..upto]);
        if flat { out.extend(g.split_whitespace().map(String::from)); } else if !g.trim().is_empty() { out.push(format!("gap:{}", g.trim())); }
    }
    *at = (*at).max(upto);
}

/// Pre-order dump without comments; leaves carry their text. Two sources are equivalent when the dumps match.
pub fn canon(n: Node, src: &[u8], out: &mut Vec<String>) { dump(n, src, out, false) }

/// `flat` keeps the token sequence only: every leaf's exact text and the words of uncovered text, no tree shape or
/// kinds (used when the source has unparsed regions, where error recovery may group the same tokens differently).
pub fn dump(n: Node, src: &[u8], out: &mut Vec<String>, flat: bool) {
    if is_comment(n) { return; }
    if n.child_count() == 0 {
        if flat { if n.end_byte() > n.start_byte() { out.push(text(n, src).to_string()); } } else { out.push(format!("{}:{}", n.kind(), text(n, src))); }
        return;
    }
    let kids = children(n);
    if kids.iter().all(|c| is_comment(*c)) && !flat { out.push(format!("{}:", n.kind())); return; }
    if !flat { out.push(format!("({}", n.kind())); }
    let mut at = n.start_byte();
    for c in kids {
        gaps(src, out, &mut at, c.start_byte(), flat);
        dump(c, src, out, flat);
        at = at.max(c.end_byte());
    }
    gaps(src, out, &mut at, n.end_byte(), flat);
    if !flat { out.push(")".into()); }
}

pub fn canon_of(spec: &Spec, s: &str) -> Option<Vec<String>> {
    let t = parse(spec, s.as_bytes());
    if first_error(t.root_node()).is_some() { return None; }
    let mut v = vec![];
    canon(t.root_node(), s.as_bytes(), &mut v);
    Some(v)
}

/// Bytes of `n` inside unparsed regions.
pub fn opaque_bytes(n: Node) -> usize {
    if is_opaque(n) { return n.end_byte() - n.start_byte(); }
    if !n.has_error() { return 0; }
    // stray tokens directly in an unparsed region are copied too
    children(n).into_iter().map(|c| if n.is_error() && c.child_count() == 0 { c.end_byte() - c.start_byte() } else { opaque_bytes(c) }).sum()
}
