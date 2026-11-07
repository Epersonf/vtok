//! Docstrings of a module keyed by qualified name, so decoding can put back what encoding dropped.
use crate::tree::{body_statements, is_docstring, named_children, text, trivia};
use std::collections::HashMap;
use tree_sitter::Node;

#[derive(Default, Clone)]
pub struct Docs {
    pub module: Option<(String, bool)>,
    pub map: HashMap<(String, usize), (String, bool)>,
}

pub(crate) fn name_of(n: Node, src: &[u8]) -> String {
    n.child_by_field_name("name").map(|x| text(x, src).to_string()).unwrap_or_default()
}

pub(crate) fn def_of<'a>(n: Node<'a>) -> Option<Node<'a>> {
    match n.kind() {
        "function_definition" | "class_definition" => Some(n),
        "decorated_definition" => n.child_by_field_name("definition"),
        _ => None,
    }
}

fn collect_walk(node: Node, src: &[u8], prefix: &str, seen: &mut HashMap<String, usize>, out: &mut HashMap<(String, usize), (String, bool)>) {
    for n in named_children(node) {
        if trivia(n) { continue; }
        if let Some(d) = def_of(n) {
            let q = format!("{prefix}{}", name_of(d, src));
            let k = { let e = seen.entry(q.clone()).and_modify(|x| *x += 1).or_insert(0); *e };
            if let Some(b) = d.child_by_field_name("body") {
                let stmts = body_statements(b, false, src);
                if stmts.first().map_or(false, |f| is_docstring(*f, src)) {
                    out.insert((q.clone(), k), (text(stmts[0], src).to_string(), stmts.len() == 1));
                }
                collect_walk(b, src, &format!("{q}."), seen, out);
            }
            // decorators / parameters may hold lambdas but never defs: nothing else to walk
        } else {
            collect_walk(n, src, prefix, seen, out);
        }
    }
}

/// Docstrings of `stmts` (a module body, or the members of a class) keyed by qualified name.
pub fn collect_docs(stmts: &[Node], src: &[u8]) -> Docs {
    let mut d = Docs::default();
    if stmts.first().map_or(false, |f| is_docstring(*f, src)) {
        d.module = Some((text(stmts[0], src).to_string(), stmts.len() == 1));
    }
    let mut seen = HashMap::new();
    for n in stmts {
        if let Some(df) = def_of(*n) {
            let q = name_of(df, src);
            let k = { let e = seen.entry(q.clone()).and_modify(|x| *x += 1).or_insert(0); *e };
            if let Some(b) = df.child_by_field_name("body") {
                let st = body_statements(b, false, src);
                if st.first().map_or(false, |f| is_docstring(*f, src)) {
                    d.map.insert((q.clone(), k), (text(st[0], src).to_string(), st.len() == 1));
                }
                collect_walk(b, src, &format!("{q}."), &mut seen, &mut d.map);
            }
        } else {
            collect_walk(*n, src, "", &mut seen, &mut d.map);
        }
    }
    d
}
