//! The units of a module for chunking: its top-level items and, for a class that is too long, its members.
use crate::tree::{named_children, parse, text};
use tree_sitter::Node;
use vtok_core::language::Unit;
use vtok_core::Result;

const CLASS_DECLS: [&str; 2] = ["class_declaration", "abstract_class_declaration"];

fn node_name(n: Node, src: &[u8]) -> String {
    let d = if n.kind() == "export_statement" { n.child_by_field_name("declaration").unwrap_or(n) } else { n };
    if let Some(nm) = d.child_by_field_name("name") { return text(nm, src).to_string(); }
    if matches!(d.kind(), "lexical_declaration" | "variable_declaration") {
        let names: Vec<String> = named_children(d).into_iter()
            .filter(|vd| vd.kind() == "variable_declarator")
            .filter_map(|vd| vd.child_by_field_name("name").map(|x| text(x, src).to_string()))
            .collect();
        if !names.is_empty() { return names.join(","); }
    }
    if d.kind() == "import_statement" { return "imports".into(); }
    d.kind().to_string()
}

fn rng(n: Node) -> (usize, usize) { (n.start_position().row + 1, n.end_position().row + 1) }

pub(super) fn units(source: &str, max_lines: usize) -> Result<Vec<Unit>> {
    let source = source.replace("\r\n", "\n");
    let src = source.as_bytes();
    let tree = parse(src);
    let mut out = vec![];
    for n in named_children(tree.root_node()) {
        if n.kind() == "comment" { continue; }
        let (s, e) = rng(n);
        let d = if n.kind() == "export_statement" { n.child_by_field_name("declaration") } else { Some(n) };
        if let Some(d) = d {
            if CLASS_DECLS.contains(&d.kind()) && e - s + 1 > max_lines {
                if let Some(body) = d.child_by_field_name("body") {
                    let cname = node_name(n, src);
                    for m in named_children(body) {
                        if m.kind() == "comment" { continue; }
                        let (ms, me) = rng(m);
                        let name = if m.child_by_field_name("name").is_some() { node_name(m, src) } else { m.kind().to_string() };
                        out.push(Unit { name, kind: m.kind().into(), start: ms, end: me, member: true, parent: Some(cname.clone()), indent: m.start_position().column });
                    }
                    continue;
                }
            }
        }
        out.push(Unit { name: node_name(n, src), kind: n.kind().into(), start: s, end: e, member: false, parent: None, indent: n.start_position().column });
    }
    Ok(out)
}
