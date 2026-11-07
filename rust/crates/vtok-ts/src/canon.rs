//! Canonical dump of a syntax tree, used to check that two sources mean the same (comments aside).
use crate::ctx::jsx_norm;
use crate::tree::{children, named_children, text};
use crate::SKIP;
use tree_sitter::Node;

const BODY_PARENTS: [&str; 6] = ["if_statement", "else_clause", "for_statement", "for_in_statement", "while_statement", "do_statement"];

pub(crate) fn canon(n: Node, src: &[u8], out: &mut Vec<String>) {
    let t = n.kind();
    if SKIP.contains(&t) { return; }
    if !n.is_named() {
        if t != ";" && t != "," { out.push(t.to_string()); }
        return;
    }
    if t == "statement_block" && n.parent().map_or(false, |p| BODY_PARENTS.contains(&p.kind())) {
        let kids: Vec<Node> = named_children(n).into_iter().filter(|c| !SKIP.contains(&c.kind())).collect();
        if kids.len() == 1 { return canon(kids[0], src, out); }
    }
    if t == "template_string" {
        out.push("(template".into());
        let mut buf = String::new();
        let mut has = false;
        let flush = |buf: &mut String, has: &mut bool, out: &mut Vec<String>| {
            if *has { out.push(format!("txt:{buf}")); buf.clear(); *has = false; }
        };
        for c in children(n) {
            match c.kind() {
                "string_fragment" | "escape_sequence" => { buf.push_str(&text(c, src).replace('\n', "\\n")); has = true; }
                "template_substitution" => { flush(&mut buf, &mut has, out); canon(c, src, out); }
                _ => {}
            }
        }
        flush(&mut buf, &mut has, out);
        out.push(")".into());
        return;
    }
    if t == "jsx_text" {
        let j = jsx_norm(text(n, src));
        if !j.is_empty() { out.push(format!("{t}:{j}")); }
        return;
    }
    if n.child_count() == 0 || t == "string" || t == "regex" {
        out.push(format!("{t}:{}", text(n, src)));
        return;
    }
    out.push(format!("({t}"));
    for c in children(n) { canon(c, src, out); }
    out.push(")".into());
}
