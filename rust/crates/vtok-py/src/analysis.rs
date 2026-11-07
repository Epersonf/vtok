//! Name census of a module: how often each identifier occurs, which names imports bind and which must not be reused.
use crate::tree::{children, named_children, text};
use crate::{IMPORTS, PY_NAMES};
use std::collections::{HashMap, HashSet};
use tree_sitter::Node;

pub struct Analysis { pub counts: HashMap<String, usize>, pub excluded: HashSet<String>, pub avoid: HashSet<String> }

fn first_ident(n: Node, src: &[u8]) -> Option<String> {
    if n.kind() == "identifier" { return Some(text(n, src).to_string()); }
    for c in children(n) { if let Some(s) = first_ident(c, src) { return Some(s); } }
    None
}

pub fn analyze(root: Node, src: &[u8]) -> Analysis {
    let mut counts: HashMap<String, usize> = HashMap::new();
    let mut excluded: HashSet<String> = HashSet::new();
    let mut avoid: HashSet<String> = PY_NAMES.lines().map(String::from).collect();
    let mut st = vec![(root, false)];
    while let Some((n, in_imp)) = st.pop() {
        let k = n.kind();
        let imp = in_imp || IMPORTS.contains(&k);
        if k == "identifier" {
            let s = text(n, src).to_string();
            avoid.insert(s.clone());
            if !imp { *counts.entry(s).or_insert(0) += 1; }
        }
        if k == "import_statement" || k == "import_from_statement" {
            let module = n.child_by_field_name("module_name").map(|m| m.id());
            for c in named_children(n) {
                if Some(c.id()) == module { continue; }
                match c.kind() {
                    "dotted_name" => { if let Some(s) = first_ident(c, src) { excluded.insert(s); } }
                    "aliased_import" => {
                        if let Some(a) = c.child_by_field_name("alias") { excluded.insert(text(a, src).to_string()); }
                    }
                    _ => {}
                }
            }
        }
        for c in children(n) { st.push((c, imp)); }
    }
    Analysis { counts, excluded, avoid }
}
