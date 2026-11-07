//! Long string literals that are replaced by short placeholders (`"$1"`, `"$2"`, ...).
use crate::tree::{children, text};
use std::collections::{BTreeMap, HashMap, HashSet};
use tree_sitter::Node;
use vtok_core::tokens::ntok;

const NO_HIDE_PARENTS: [&str; 5] = ["import_statement", "export_statement", "expression_statement", "literal_type", "import_require_clause"];
const NO_HIDE_ANCESTORS: [&str; 7] = [
    "type_annotation", "type_alias_declaration", "interface_declaration", "literal_type",
    "import_statement", "export_statement", "ambient_declaration",
];

pub(crate) fn collect_literals(root: Node, src: &[u8], minimum: usize) -> (HashMap<usize, String>, BTreeMap<String, String>) {
    let mut strings: Vec<Node> = vec![];
    let mut existing: HashSet<String> = HashSet::new();
    let mut st = vec![root];
    while let Some(n) = st.pop() {
        for c in children(n) { st.push(c); }
        if n.kind() == "string" {
            existing.insert(text(n, src).to_string());
            let mut p = n.parent();
            let mut ok = true;
            while let Some(a) = p {
                let k = a.kind();
                let is_direct = n.parent().map_or(false, |q| q.id() == a.id());
                if NO_HIDE_ANCESTORS.contains(&k) && !(k == "export_statement" && !is_direct) { ok = false; break; }
                p = a.parent();
            }
            let par_ok = n.parent().map_or(true, |q| !NO_HIDE_PARENTS.contains(&q.kind()));
            if ok && par_ok && ntok(text(n, src)) >= minimum { strings.push(n); }
        }
    }
    strings.sort_by_key(|n| n.start_byte());
    let (mut pos, mut back) = (HashMap::new(), BTreeMap::new());
    let mut by_text: HashMap<String, String> = HashMap::new();
    let mut k = 0usize;
    for n in strings {
        let s = text(n, src).to_string();
        if !by_text.contains_key(&s) {
            k += 1;
            while existing.contains(&format!("\"${k}\"")) { k += 1; }
            let ph = format!("\"${k}\"");
            by_text.insert(s.clone(), ph.clone());
            back.insert(ph, s.clone());
        }
        pos.insert(n.start_byte(), by_text[&s].clone());
    }
    (pos, back)
}
