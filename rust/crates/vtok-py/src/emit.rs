//! Statement emission: a syntax node becomes tokens (`flatten`), tokens become `Line`s (`emit_stmt`), lines become text
//! (`render`).
use crate::ctx::{is_field, join, literal_placeholder, rename_of, text_renamed, Ctx, Line};
use crate::tree::{body_statements, children, has_interpolation, is_plain_str, text, trivia};
use crate::{CLAUSES, IMPORTS};
use tree_sitter::Node;
use vtok_core::tokens::ntok;

pub(crate) fn flatten(n: Node, ctx: &Ctx, in_import: bool) -> Vec<String> {
    let t = n.kind();
    if trivia(n) { return vec![]; }
    match t {
        "identifier" => {
            let s = text(n, ctx.src);
            return vec![if in_import { s.to_string() } else { rename_of(s, ctx) }];
        }
        "string" => {
            if has_interpolation(n) { return vec![text_renamed(n, ctx)]; }
            let s = text(n, ctx.src);
            if ctx.hide > 0 && is_plain_str(n, ctx.src) && ntok(s) >= ctx.hide { return vec![literal_placeholder(s, ctx)]; }
            return vec![s.to_string()];
        }
        _ => {}
    }
    let imp = in_import || IMPORTS.contains(&t);
    if n.child_count() == 0 { return vec![text(n, ctx.src).to_string()]; }
    let mut out: Vec<String> = vec![];
    for c in children(n) {
        if ctx.lean && (is_field(n, c, "type") || is_field(n, c, "return_type")) {
            if matches!(out.last().map(|s| s.as_str()), Some(":") | Some("->")) { out.pop(); }
            continue;
        }
        out.extend(flatten(c, ctx, imp));
    }
    out
}

pub(crate) fn simple_line(n: Node, ctx: &Ctx, indent: usize) -> Line {
    Line { indent, text: join(&flatten(n, ctx, false)), simple: true }
}

pub(crate) fn emit_stmt(n: Node, ctx: &Ctx, indent: usize, out: &mut Vec<Line>) {
    let t = n.kind();
    if trivia(n) { return; }
    if t == "decorated_definition" {
        for c in children(n) {
            if trivia(c) { continue; }
            if c.kind() == "decorator" {
                out.push(Line { indent, text: join(&flatten(c, ctx, false)), simple: false });
            } else {
                emit_stmt(c, ctx, indent, out);
            }
        }
        return;
    }
    let compound = matches!(t, "if_statement" | "for_statement" | "while_statement" | "try_statement" | "with_statement"
        | "function_definition" | "class_definition" | "match_statement") || CLAUSES.contains(&t);
    if !compound { out.push(simple_line(n, ctx, indent)); return; }
    let is_def = matches!(t, "function_definition" | "class_definition");
    let mut header: Vec<String> = vec![];
    for c in children(n) {
        if trivia(c) { continue; }
        let k = c.kind();
        if k == "block" {
            let mut body: Vec<Line> = vec![];
            if ctx.skeleton && t == "function_definition" {
                body.push(Line { indent: indent + 1, text: "...".into(), simple: true });
            } else {
                for s in body_statements(c, is_def, ctx.src) { emit_stmt(s, ctx, indent + 1, &mut body); }
                if body.is_empty() { body.push(Line { indent: indent + 1, text: "pass".into(), simple: true }); }
            }
            let head = join(&header);
            header.clear();
            if !is_def && body.iter().all(|l| l.simple) {
                let joined = body.iter().map(|l| l.text.as_str()).collect::<Vec<_>>().join(";");
                out.push(Line { indent, text: head + &joined, simple: false });
            } else {
                out.push(Line { indent, text: head, simple: false });
                out.extend(body);
            }
        } else if CLAUSES.contains(&k) {
            if !header.is_empty() { out.push(Line { indent, text: join(&header), simple: false }); header.clear(); }
            emit_stmt(c, ctx, indent, out);
        } else if ctx.lean && is_field(n, c, "return_type") {
            if header.last().map(|s| s.as_str()) == Some("->") { header.pop(); }
        } else {
            header.extend(flatten(c, ctx, false));
        }
    }
    if !header.is_empty() { out.push(Line { indent, text: join(&header), simple: false }); }
}

pub(crate) fn render(lines: Vec<Line>) -> String {
    let mut res: Vec<Line> = vec![];
    for l in lines {
        if let Some(last) = res.last_mut() {
            if last.indent == l.indent && last.simple && l.simple {
                last.text.push(';');
                last.text.push_str(&l.text);
                continue;
            }
        }
        res.push(l);
    }
    let mut s = String::new();
    for l in res { s.push_str(&" ".repeat(l.indent)); s.push_str(&l.text); s.push('\n'); }
    s
}
