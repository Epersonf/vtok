//! Identifier spans of a whole Rust file for name-based search and the relation graph: every leaf token outside
//! comments and string / char literals (macro bodies included). Tolerates syntax errors (tree-sitter recovers).
use super::parse;

const SKIP: &[&str] = &["line_comment", "block_comment", "string_literal", "raw_string_literal", "char_literal"];

pub(crate) fn spans(text: &str) -> Vec<(usize, usize)> {
    let tree = parse(text.as_bytes());
    let mut out = vec![];
    let mut st = vec![tree.root_node()];
    while let Some(n) = st.pop() {
        if SKIP.contains(&n.kind()) { continue; }
        if n.child_count() == 0 {
            if n.end_byte() > n.start_byte() { out.push((n.start_byte(), n.end_byte())); }
            continue;
        }
        let mut c = n.walk();
        let kids: Vec<_> = n.children(&mut c).collect();
        for k in kids.into_iter().rev() { st.push(k); }
    }
    out
}
