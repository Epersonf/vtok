//! The units of a module for chunking: its items (attributes join the item after them) and, for an impl, trait or mod
//! that is too long, its members.
use crate::tree::{named_children, parse, text};
use crate::COMMENTS;
use tree_sitter::Node;
use vtok_core::language::Unit;
use vtok_core::Result;

const BODY_ITEMS: [&str; 3] = ["impl_item", "trait_item", "mod_item"];

fn rng(n: Node) -> (usize, usize) { (n.start_position().row + 1, n.end_position().row + 1) }

fn base_type<'a>(n: Node<'a>, src: &[u8]) -> String {
    let n = if n.kind() == "generic_type" { n.child_by_field_name("type").unwrap_or(n) } else { n };
    text(n, src).to_string()
}

fn item_name(n: Node, src: &[u8]) -> String {
    match n.kind() {
        "impl_item" => {
            let ty = n.child_by_field_name("type").map(|t| base_type(t, src)).unwrap_or_default();
            match n.child_by_field_name("trait") {
                Some(tr) => format!("{} for {ty}", base_type(tr, src)),
                None => ty,
            }
        }
        "use_declaration" => "use".into(),
        "extern_crate_declaration" => "extern".into(),
        "attribute_item" | "inner_attribute_item" => "attr".into(),
        "expression_statement" | "macro_invocation" => {
            let m = if n.kind() == "macro_invocation" { Some(n) } else { named_children(n).into_iter().find(|c| c.kind() == "macro_invocation") };
            m.and_then(|m| m.child_by_field_name("macro")).map(|x| format!("{}!", text(x, src))).unwrap_or_else(|| n.kind().to_string())
        }
        k => n.child_by_field_name("name").map(|x| text(x, src).to_string()).unwrap_or_else(|| k.to_string()),
    }
}

/// Items of `nodes` as units; attributes join the item that follows them.
fn push_units(nodes: &[Node], src: &[u8], max_lines: usize, parent: Option<&str>, out: &mut Vec<Unit>) {
    let mut pending: Option<Node> = None;
    for &n in nodes {
        if COMMENTS.contains(&n.kind()) { continue; }
        if matches!(n.kind(), "attribute_item") {
            if pending.is_none() { pending = Some(n); }
            continue;
        }
        let first = pending.take().unwrap_or(n);
        let (s, e) = (first.start_position().row + 1, n.end_position().row + 1);
        let member = parent.is_some();
        if !member && BODY_ITEMS.contains(&n.kind()) && e - s + 1 > max_lines {
            if let Some(body) = n.child_by_field_name("body") {
                let name = item_name(n, src);
                push_units(&named_children(body), src, max_lines, Some(&name), out);
                continue;
            }
        }
        out.push(Unit {
            name: item_name(n, src), kind: n.kind().into(), start: s, end: e, member,
            parent: parent.map(String::from), indent: first.start_position().column,
        });
    }
    if let Some(p) = pending {
        let (s, e) = rng(p);
        out.push(Unit { name: "attr".into(), kind: p.kind().into(), start: s, end: e, member: parent.is_some(), parent: parent.map(String::from), indent: p.start_position().column });
    }
}

pub(super) fn units(source: &str, max_lines: usize) -> Result<Vec<Unit>> {
    let source = source.replace("\r\n", "\n");
    let tree = parse(source.as_bytes());
    let mut out = vec![];
    push_units(&named_children(tree.root_node()), source.as_bytes(), max_lines, None, &mut out);
    Ok(out)
}
