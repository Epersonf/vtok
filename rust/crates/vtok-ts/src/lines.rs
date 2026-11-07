//! Top-level statements as notation lines: one line per statement, with the `;` between lines dropped where safe.
use crate::ctx::{join, Ctx};
use crate::flat::flat;
use crate::SKIP;
use tree_sitter::Node;
use vtok_core::Result;

fn flat_stmt(n: Node, ctx: &Ctx) -> Result<Vec<String>> {
    let toks = flat(n, ctx)?;
    let mut out = Vec::with_capacity(toks.len());
    for i in 0..toks.len() {
        if toks[i] == ";" && i + 1 < toks.len() && toks[i + 1] == "}" { continue; }
        out.push(toks[i].clone());
    }
    Ok(out)
}

pub(crate) fn native_lines(nodes: &[Node], ctx: &Ctx) -> Result<Vec<String>> {
    let mut lines = vec![];
    for c in nodes {
        if SKIP.contains(&c.kind()) { continue; }
        lines.push(join(&flat_stmt(*c, ctx)?));
    }
    for i in 0..lines.len() {
        let nxt: String = if i + 1 < lines.len() { lines[i + 1].clone() } else { String::new() };
        if lines[i].ends_with(';') {
            let first = nxt.chars().next();
            if first.map_or(true, |c| c.is_alphabetic() || c == '_' || c == '$' || c == '@') {
                lines[i].pop();
            }
        }
    }
    Ok(lines)
}
