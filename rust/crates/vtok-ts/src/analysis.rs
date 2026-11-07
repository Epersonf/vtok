//! Name census of a module: how often each identifier occurs, which names must keep their spelling (imports, exports,
//! shorthand properties) and which must not be reused.
use crate::tree::{children, named_children, text};
use crate::is_ident;
use std::collections::{HashMap, HashSet};
use tree_sitter::Node;

const LIKE_IDENT: [&str; 7] = [
    "identifier", "type_identifier", "property_identifier", "shorthand_property_identifier",
    "shorthand_property_identifier_pattern", "statement_identifier", "private_property_identifier",
];
const JS_WORDS: &str = "abstract any as async await boolean break case catch class const continue debugger declare default delete do else enum export extends false finally for from function get if implements import in instanceof interface is keyof let module namespace never new null number object of override package private protected public readonly return satisfies set static string super switch symbol this throw true try type typeof undefined unknown var void while with yield";
const KW_VALUES: &str = "f C I k l v m r R B K t x y S L w E e Y X";

fn idents(n: Node, src: &[u8], out: &mut HashSet<String>) {
    if is_ident(n.kind()) { out.insert(text(n, src).to_string()); }
    for c in children(n) { idents(c, src, out); }
}

pub struct Analysis {
    pub counts: HashMap<String, usize>,
    pub excl: HashSet<String>,
    pub avoid: HashSet<String>,
}

pub(crate) fn words(s: &str) -> impl Iterator<Item = String> + '_ { s.split_whitespace().map(String::from) }

pub fn analyze(root: Node, src: &[u8]) -> Analysis {
    let mut counts: HashMap<String, usize> = HashMap::new();
    let mut excl: HashSet<String> = HashSet::new();
    let mut avoid: HashSet<String> = words(JS_WORDS).chain(words(KW_VALUES)).collect();
    avoid.insert("?".into());
    avoid.insert("e".into());
    let mut st = vec![root];
    while let Some(n) = st.pop() {
        for c in children(n) { st.push(c); }
        let t = n.kind();
        if LIKE_IDENT.contains(&t) { avoid.insert(text(n, src).to_string()); }
        if is_ident(t) {
            *counts.entry(text(n, src).to_string()).or_insert(0) += 1;
        } else if t == "shorthand_property_identifier" || t == "shorthand_property_identifier_pattern" {
            excl.insert(text(n, src).to_string());
        } else if t == "import_statement" {
            idents(n, src, &mut excl);
        } else if t == "export_statement" {
            for c in children(n) {
                if is_ident(c.kind()) { excl.insert(text(c, src).to_string()); }
                else if c.kind() == "export_clause" { idents(c, src, &mut excl); }
            }
            if let Some(d) = n.child_by_field_name("declaration") {
                if let Some(nm) = d.child_by_field_name("name") { idents(nm, src, &mut excl); }
                for vd in named_children(d) {
                    if vd.kind() == "variable_declarator" {
                        if let Some(nm) = vd.child_by_field_name("name") { idents(nm, src, &mut excl); }
                    }
                }
            }
        }
    }
    Analysis { counts, excl, avoid }
}
