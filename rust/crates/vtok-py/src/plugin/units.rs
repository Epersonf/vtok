//! The units of a module for chunking: its top-level items and, for a class that is too long, its members.
use super::wrap::body_of;
use crate::docs::{def_of, name_of};
use crate::tree::{is_docstring, named_children, parse, text, trivia};
use tree_sitter::Node;
use vtok_core::language::Unit;
use vtok_core::Result;

fn unit_label(n: Node, src: &[u8]) -> String {
    let d = def_of(n);
    if let Some(d) = d { return name_of(d, src); }
    match n.kind() {
        "import_statement" | "import_from_statement" | "future_import_statement" => "imports".into(),
        "expression_statement" => {
            let kids = named_children(n);
            if let Some(a) = kids.first() {
                if matches!(a.kind(), "assignment" | "augmented_assignment") {
                    let mut names = vec![];
                    let mut cur = *a;
                    loop {
                        if let Some(l) = cur.child_by_field_name("left") {
                            if l.kind() == "identifier" { names.push(text(l, src).to_string()); }
                        }
                        match cur.child_by_field_name("right") {
                            Some(r) if r.kind() == "assignment" => cur = r,
                            _ => break,
                        }
                    }
                    return if names.is_empty() { "assign".into() } else { names.join(",") };
                }
            }
            "code".into()
        }
        _ => "code".into(),
    }
}

fn unit_kind(n: Node) -> String { n.kind().to_string() }

pub(super) fn units(source: &str, max_lines: usize) -> Result<Vec<Unit>> {
    let source = source.replace("\r\n", "\n");
    let src = source.as_bytes();
    let tree = parse(src);
    let mut out = vec![];
    for n in body_of(tree.root_node(), false) {
        let (s, e) = (n.start_position().row + 1, n.end_position().row + 1);
        let class = def_of(n).filter(|d| d.kind() == "class_definition");
        if let (Some(c), true) = (class, e - s + 1 > max_lines) {
            let cname = name_of(c, src);
            let body = c.child_by_field_name("body").map(|b| named_children(b).into_iter().filter(|x| !trivia(*x)).collect::<Vec<_>>()).unwrap_or_default();
            for (i, m) in body.iter().enumerate() {
                if i == 0 && is_docstring(*m, src) { continue; } // class docstring stays in the shell
                out.push(Unit {
                    name: unit_label(*m, src), kind: unit_kind(*m),
                    start: m.start_position().row + 1, end: m.end_position().row + 1,
                    member: true, parent: Some(cname.clone()), indent: m.start_position().column,
                });
            }
        } else {
            out.push(Unit { name: unit_label(n, src), kind: unit_kind(n), start: s, end: e, member: false, parent: None, indent: n.start_position().column });
        }
    }
    Ok(out)
}
