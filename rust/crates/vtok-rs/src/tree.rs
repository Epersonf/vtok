//! Parsing (with the `&raw` workaround), the first syntax error and the node helpers shared by every part of the plugin.
use crate::IDENT;
use tree_sitter::{Node, Parser, Tree};

pub fn parse(src: &[u8]) -> Tree {
    let mut p = Parser::new();
    p.set_language(&tree_sitter_rust::LANGUAGE.into()).expect("rust grammar");
    match mask_raw_ident(src) {
        Some(m) => p.parse(&m, None).expect("parse"),
        None => p.parse(src, None).expect("parse"),
    }
}

/// tree-sitter-rust reads `&raw` as the start of `&raw const/mut`, so a variable named `raw`
/// (`f(&raw)`) fails to parse. Mask such an identifier with one of equal length (offsets stay valid).
fn mask_raw_ident(src: &[u8]) -> Option<Vec<u8>> {
    let mut out: Option<Vec<u8>> = None;
    let mut i = 0;
    while i + 4 <= src.len() {
        if &src[i..i + 3] == b"raw" && (i == 0 || src[i - 1] == b'&' || src[i - 1].is_ascii_whitespace()) {
            let mut b = i;
            while b > 0 && src[b - 1].is_ascii_whitespace() { b -= 1; }
            let after = src[i + 3];
            let ident_end = after.is_ascii_alphanumeric() || after == b'_';
            if b > 0 && src[b - 1] == b'&' && !ident_end {
                let mut j = i + 3;
                while j < src.len() && src[j].is_ascii_whitespace() { j += 1; }
                let rest = &src[j..];
                let kw = (rest.starts_with(b"const") || rest.starts_with(b"mut"))
                    && !rest.get(if rest.starts_with(b"const") { 5 } else { 3 }).map_or(false, |c| c.is_ascii_alphanumeric() || *c == b'_');
                if !kw { out.get_or_insert_with(|| src.to_vec())[i + 2] = b'x'; }
            }
        }
        i += 1;
    }
    out
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

/// An identifier that the alias table may rename (lifetimes and labels keep their names).
pub(crate) fn renamable(n: Node) -> bool {
    IDENT.contains(&n.kind()) && n.parent().map_or(true, |p| !matches!(p.kind(), "lifetime" | "label"))
}
