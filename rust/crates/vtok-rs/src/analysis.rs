//! Name census of a module: how often each identifier occurs, which names must keep their spelling (public items,
//! format-string captures, macros) and which must not be reused.
use crate::ctx::isw;
use crate::tree::{children, named_children, renamable, text};
use crate::IDENT;
use std::collections::{HashMap, HashSet};
use tree_sitter::Node;

const RUST_WORDS: &str = "abstract as async await become box break const continue crate do dyn else enum extern false final fn for gen if impl in let loop macro match mod move mut override priv pub ref return self Self static struct super trait true try type typeof union unsafe unsized use virtual where while yield bool char str u8 u16 u32 u64 u128 usize i8 i16 i32 i64 i128 isize f32 f64 Some None Ok Err Option Result Vec String Box Default main r b br";

pub struct Analysis {
    pub counts: HashMap<String, usize>,
    pub excl: HashSet<String>,
    pub avoid: HashSet<String>,
}

fn add_idents(n: Node, src: &[u8], out: &mut HashSet<String>) {
    if IDENT.contains(&n.kind()) { out.insert(text(n, src).to_string()); }
    for c in children(n) { add_idents(c, src, out); }
}

fn add_name(n: Node, field: &str, src: &[u8], out: &mut HashSet<String>) {
    if let Some(x) = n.child_by_field_name(field) { add_idents(x, src, out); }
}

fn has_vis(n: Node) -> bool { children(n).iter().any(|c| c.kind() == "visibility_modifier") }

fn words(s: &str) -> impl Iterator<Item = String> + '_ { s.split_whitespace().map(String::from) }

/// `{name}` / `{name:?}` captures inside format strings refer to variables by their real name.
fn format_captures(s: &str, out: &mut HashSet<String>) {
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'{' {
            let st = i + 1;
            let mut j = st;
            while j < b.len() && (b[j].is_ascii_alphanumeric() || b[j] == b'_') { j += 1; }
            if j > st && !b[st].is_ascii_digit() { out.insert(s[st..j].to_string()); }
            i = j.max(i + 1);
        } else { i += 1; }
    }
}

pub fn analyze(root: Node, src: &[u8]) -> Analysis {
    let mut counts: HashMap<String, usize> = HashMap::new();
    let mut excl: HashSet<String> = HashSet::new();
    let mut avoid: HashSet<String> = words(RUST_WORDS).collect();
    avoid.insert("e".into());
    let mut st = vec![root];
    while let Some(n) = st.pop() {
        for c in children(n) { st.push(c); }
        let t = n.kind();
        if n.child_count() == 0 || IDENT.contains(&t) {
            let s = text(n, src);
            if s.chars().next().map_or(false, |c| c.is_alphabetic() || c == '_') && s.chars().all(isw) { avoid.insert(s.to_string()); }
        }
        if renamable(n) {
            let s = text(n, src);
            *counts.entry(s.to_string()).or_insert(0) += 1;
            if s.starts_with("r#") { excl.insert(s.to_string()); }
        }
        match t {
            "string_literal" => format_captures(text(n, src), &mut excl),
            "use_declaration" | "extern_crate_declaration" | "attribute_item" | "inner_attribute_item" | "foreign_mod_item" => add_idents(n, src, &mut excl),
            "macro_invocation" => add_name(n, "macro", src, &mut excl),
            "macro_definition" | "mod_item" => add_name(n, "name", src, &mut excl),
            "function_item" | "struct_item" | "enum_item" | "trait_item" | "const_item" | "static_item" | "type_item" | "union_item" | "field_declaration" | "function_signature_item" => {
                if has_vis(n) || t == "trait_item" { add_name(n, "name", src, &mut excl); }
                if t == "enum_item" && has_vis(n) {
                    if let Some(b) = n.child_by_field_name("body") {
                        for v in named_children(b) { if v.kind() == "enum_variant" { add_name(v, "name", src, &mut excl); } }
                    }
                }
                if t == "trait_item" {
                    if let Some(b) = n.child_by_field_name("body") {
                        for m in named_children(b) { add_name(m, "name", src, &mut excl); }
                    }
                }
            }
            "impl_item" => {
                if n.child_by_field_name("trait").is_some() {
                    if let Some(b) = n.child_by_field_name("body") {
                        for m in named_children(b) { add_name(m, "name", src, &mut excl); }
                    }
                }
            }
            _ => {}
        }
    }
    excl.insert("Self".into());
    Analysis { counts, excl, avoid }
}
