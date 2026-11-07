//! A syntax node as a list of tokens (`flat`), with template strings and the `;` that ASI-style source leaves out.
use crate::ctx::{join, jsx_norm, Ctx};
use crate::tree::{children, text};
use crate::is_ident;
use tree_sitter::Node;
use vtok_core::{Result, VtokError};

pub(crate) fn flat(n: Node, ctx: &Ctx) -> Result<Vec<String>> {
    let t = n.kind();
    if t == "comment" { return Ok(vec![]); }
    if t == "string" {
        if let Some(p) = ctx.lits.get(&n.start_byte()) { return Ok(vec![p.clone()]); }
    }
    if ctx.lean {
        if t == "type_annotation" { return Ok(vec![]); }
        if t == "import_statement" {
            let mut ids: Vec<String> = vec![];
            let mut st = vec![n];
            while let Some(x) = st.pop() {
                let ch = children(x);
                for c in ch.into_iter().rev() { st.push(c); }
                if x.kind() == "identifier" { ids.push(text(x, ctx.src).to_string()); }
            }
            return Ok(if ids.is_empty() { vec![] } else { vec![format!("import{{{}}}", ids.join(",")), ";".to_string()] });
        }
    }
    if is_ident(t) {
        let s = text(n, ctx.src);
        return Ok(vec![ctx.ren.get(s).cloned().unwrap_or_else(|| s.to_string())]);
    }
    if t == "template_string" { return Ok(vec![template(n, ctx)?]); }
    if ctx.skeleton && matches!(t, "arrow_function" | "function_expression" | "function" | "generator_function" | "function_declaration" | "generator_function_declaration" | "method_definition") {
        if let Some(b) = n.child_by_field_name("body") {
            if b.kind() == "statement_block" {
                let mut out = vec![];
                for c in children(n) {
                    if c.start_byte() < b.start_byte() && c.kind() != "comment" { out.extend(flat(c, ctx)?); }
                }
                out.push("{}".to_string());
                return Ok(out);
            }
        }
    }
    if t == "template_literal_type" {
        let mut parts = String::new();
        for c in children(n) {
            if c.kind() == "template_type" {
                let mut inner = vec![];
                for ch in children(c) {
                    if ch.kind() != "${" && ch.kind() != "}" { inner.extend(flat(ch, ctx)?); }
                }
                parts.push_str("${");
                parts.push_str(&join(&inner));
                parts.push('}');
            } else {
                parts.push_str(text(c, ctx.src));
            }
        }
        return Ok(vec![parts]);
    }
    if t == "jsx_text" {
        let j = jsx_norm(text(n, ctx.src));
        return Ok(if j.is_empty() { vec![] } else { vec![j] });
    }
    if n.child_count() == 0 || t == "string" || t == "regex" {
        let s = text(n, ctx.src);
        if s.contains('\n') { return Err(VtokError::new(format!("unsupported: newline inside {t}"))); }
        return Ok(vec![s.to_string()]);
    }
    let mut out = vec![];
    for c in children(n) { out.extend(flat(c, ctx)?); }
    // Source relying on ASI (no `;`): inline blocks need an explicit separator.
    if TERMINATED.contains(&t) && out.last().map_or(false, |l| l != ";") && !text(n, ctx.src).ends_with(';') {
        out.push(";".to_string());
    }
    if MEMBERS.contains(&t) && n.next_sibling().map_or(false, |s| s.kind() != ";" && s.kind() != ",") {
        out.push(";".to_string());
    }
    Ok(out)
}

const TERMINATED: &[&str] = &["expression_statement", "lexical_declaration", "variable_declaration", "return_statement", "throw_statement", "break_statement", "continue_statement", "debugger_statement", "do_statement", "public_field_definition", "type_alias_declaration", "import_statement"];

const MEMBERS: &[&str] = &["property_signature", "method_signature", "call_signature", "construct_signature", "index_signature", "abstract_method_signature"];

fn template(n: Node, ctx: &Ctx) -> Result<String> {
    let tagged = n.parent().map_or(false, |p| p.kind() == "call_expression");
    let mut parts = String::new();
    for c in children(n) {
        if c.kind() == "template_substitution" {
            let mut inner = vec![];
            for ch in children(c) {
                if ch.kind() != "${" && ch.kind() != "}" { inner.extend(flat(ch, ctx)?); }
            }
            parts.push_str("${");
            parts.push_str(&join(&inner));
            parts.push('}');
        } else {
            let s = text(c, ctx.src);
            if s.contains('\n') {
                if tagged { return Err(VtokError::new("unsupported: multiline tagged template")); }
                parts.push_str(&s.replace('\n', "\\n"));
            } else {
                parts.push_str(s);
            }
        }
    }
    Ok(parts)
}
