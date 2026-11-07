//! Canonical dump of a syntax tree, used to check that two sources mean the same (comments aside).
use crate::tree::{children, text};
use crate::{LEAF_TEXT, SKIP};
use tree_sitter::Node;

pub(crate) fn canon(n: Node, src: &[u8], out: &mut Vec<String>) {
    let t = n.kind();
    if SKIP.contains(&t) { return; }
    if !n.is_named() {
        out.push(t.to_string());
        return;
    }
    if (n.child_count() == 0 && t != "source_file") || LEAF_TEXT.contains(&t) {
        out.push(format!("{t}:{}", text(n, src)));
        return;
    }
    out.push(format!("({t}"));
    for c in children(n) { canon(c, src, out); }
    out.push(")".into());
}
