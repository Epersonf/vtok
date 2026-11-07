//! A syntax node as a list of tokens (`flat`), and the top-level nodes as notation lines.
use crate::ctx::{join, Ctx};
use crate::tree::{children, renamable, text};
use crate::{COMMENTS, LEAF_TEXT, SKIP, STRINGS};
use tree_sitter::Node;
use vtok_core::Result;

fn is_name(p: Option<Node>, n: Node, field: &str) -> bool {
    p.and_then(|p| p.child_by_field_name(field)).map_or(false, |x| x.id() == n.id())
}

/// Identifiers a `use` keeps in the lean view: the last segment of every path.
fn use_names(n: Node, ctx: &Ctx, out: &mut Vec<String>) {
    if n.kind() == "identifier" {
        let p = n.parent();
        let is_path = p.map_or(false, |p| matches!(p.kind(), "scoped_identifier" | "scoped_use_list") && is_name(Some(p), n, "path"));
        if !is_path {
            let s = text(n, ctx.src);
            out.push(ctx.ren.get(s).cloned().unwrap_or_else(|| s.to_string()));
        }
    }
    for c in children(n) { use_names(c, ctx, out); }
}

fn flat(n: Node, ctx: &Ctx) -> Result<Vec<String>> {
    let t = n.kind();
    if COMMENTS.contains(&t) { return Ok(vec![]); }
    if STRINGS.contains(&t) {
        if let Some(p) = ctx.lits.get(&n.start_byte()) { return Ok(vec![p.clone()]); }
    }
    if LEAF_TEXT.contains(&t) { return Ok(vec![text(n, ctx.src).to_string()]); }
    if renamable(n) {
        let s = text(n, ctx.src);
        return Ok(vec![ctx.ren.get(s).cloned().unwrap_or_else(|| s.to_string())]);
    }
    if ctx.lean {
        if t == "use_declaration" {
            let mut ids = vec![];
            use_names(n, ctx, &mut ids);
            return Ok(if ids.is_empty() { vec![] } else { vec![format!("use{{{}}}", ids.join(",")), ";".to_string()] });
        }
        if matches!(t, "let_declaration" | "parameter" | "function_item") {
            let drop = n.child_by_field_name(if t == "function_item" { "return_type" } else { "type" });
            if let Some(d) = drop {
                let kids = children(n);
                let mut out = vec![];
                for (i, c) in kids.iter().enumerate() {
                    let before_drop = kids.get(i + 1).map_or(false, |k| k.id() == d.id()) && matches!(c.kind(), ":" | "->");
                    if c.id() == d.id() || before_drop { continue; }
                    out.extend(flat(*c, ctx)?);
                }
                return Ok(out);
            }
        }
    }
    if ctx.skeleton && t == "function_item" {
        if let Some(b) = n.child_by_field_name("body") {
            let mut out = vec![];
            for c in children(n) {
                if c.start_byte() < b.start_byte() { out.extend(flat(c, ctx)?); }
            }
            out.push("{}".to_string());
            return Ok(out);
        }
    }
    if n.child_count() == 0 { return Ok(vec![text(n, ctx.src).to_string()]); }
    let mut out = vec![];
    for c in children(n) { out.extend(flat(c, ctx)?); }
    Ok(out)
}

pub(crate) fn native_lines(nodes: &[Node], ctx: &Ctx) -> Result<Vec<String>> {
    let mut lines = vec![];
    for c in nodes {
        if SKIP.contains(&c.kind()) { continue; }
        lines.push(join(&flat(*c, ctx)?));
    }
    Ok(lines)
}
