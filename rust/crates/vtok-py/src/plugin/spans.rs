//! Byte spans of tokens and identifiers in a text, for counting and renaming without a full encode.
use super::wrap::wrapped;
use crate::tree::{first_error, parse};

pub(super) fn tokens(text: &str, member: bool) -> Option<Vec<(usize, usize)>> {
    let w = wrapped(text, member);
    let off = w.len() - text.len();
    let tree = parse(w.as_bytes());
    if first_error(tree.root_node()).is_some() { return None; }
    let mut out = vec![];
    let mut st = vec![tree.root_node()];
    while let Some(n) = st.pop() {
        if n.kind() == "comment" { continue; }
        if n.child_count() == 0 || n.kind() == "string" {
            let (a, b) = (n.start_byte(), n.end_byte());
            if b > a && a >= off && b - off <= text.len() && !w[a..b].trim().is_empty() { out.push((a - off, b - off)); }
        } else {
            let mut c = n.walk();
            let kids: Vec<_> = n.children(&mut c).collect();
            for k in kids.into_iter().rev() { st.push(k); }
        }
    }
    Some(out)
}

pub(super) fn identifier_spans(text: &str) -> Option<Vec<(usize, usize)>> {
    let tree = parse(text.as_bytes());
    if first_error(tree.root_node()).is_some() { return None; }
    let mut out = vec![];
    let mut st = vec![tree.root_node()];
    while let Some(n) = st.pop() {
        match n.kind() {
            "comment" | "string" => continue,
            "identifier" => { out.push((n.start_byte(), n.end_byte())); continue; }
            _ => {}
        }
        let mut c = n.walk();
        let kids: Vec<_> = n.children(&mut c).collect();
        for k in kids { st.push(k); }
    }
    out.sort();
    Some(out)
}
