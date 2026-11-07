//! Byte spans of the items and tokens of a text, for counting and renaming without a full encode.
use super::wrap::{wrapped, wrapper_members};
use crate::tree::{first_error, named_children, parse};
use crate::SKIP;

pub(super) fn item_spans(text: &str, member: bool) -> Option<Vec<(usize, usize)>> {
    let w = wrapped(text, member);
    let off = if member { w.len() - text.len() - 2 } else { 0 };
    let tree = parse(w.as_bytes());
    if first_error(tree.root_node()).is_some() { return None; }
    let nodes = if member { wrapper_members(tree.root_node()) } else { named_children(tree.root_node()) };
    Some(nodes.into_iter().filter(|n| !SKIP.contains(&n.kind())).map(|n| (n.start_byte() - off, n.end_byte() - off)).collect())
}

pub(super) fn tokens(text: &str, member: bool) -> Option<Vec<(usize, usize)>> {
    let w = wrapped(text, member);
    let off = if member { w.len() - text.len() - 2 } else { 0 };
    let tree = parse(w.as_bytes());
    if first_error(tree.root_node()).is_some() { return None; }
    let mut out = vec![];
    let mut st = vec![tree.root_node()];
    while let Some(n) = st.pop() {
        if SKIP.contains(&n.kind()) { continue; }
        if n.child_count() == 0 {
            if n.end_byte() > n.start_byte() && n.start_byte() >= off && n.end_byte() - off <= text.len() { out.push((n.start_byte() - off, n.end_byte() - off)); }
        } else {
            let mut c = n.walk();
            let kids: Vec<_> = n.children(&mut c).collect();
            for k in kids.into_iter().rev() { st.push(k); }
        }
    }
    Some(out)
}
