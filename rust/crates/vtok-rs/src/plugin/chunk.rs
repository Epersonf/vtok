//! Chunks: encode with a fixed alias table, decode a chunk back to source, and check that a chunk round-trips.
use super::wrap::{wrapped, wrapper_members, WRAP};
use crate::canon::canon;
use crate::codec::{decode, encode};
use crate::ctx::Ctx;
use crate::flat::native_lines;
use crate::tree::{first_error, parse, text};
use crate::{EncodeOpts, COMMENTS};
use std::collections::{BTreeMap, HashMap};
use vtok_core::{Result, VtokError};

pub(super) fn encode_chunk(text: &str, table: &BTreeMap<String, String>, member: bool) -> Result<String> {
    let opts = EncodeOpts { table: Some(table.clone()), ..Default::default() };
    if !member { return Ok(encode(text, &opts)?.vtok); }
    let w = wrapped(text, true);
    let src = w.as_bytes();
    let tree = parse(src);
    if let Some(e) = first_error(tree.root_node()) {
        return Err(VtokError::new(format!("source has syntax errors near line {}", e.start_position().row)));
    }
    let ctx = Ctx { ren: table, skeleton: false, lean: false, lits: &HashMap::new(), src };
    let members = wrapper_members(tree.root_node());
    Ok(native_lines(&members, &ctx)?.join("\n") + "\n")
}

pub(super) fn decode_chunk(vtok: &str, inv: &HashMap<String, String>, member: bool, indent: usize, _original: Option<&str>) -> Result<String> {
    let none = HashMap::new();
    if !member { return decode(vtok, inv, &none); }
    let out = decode(&format!("mod {WRAP}{{\n{vtok}\n}}\n"), inv, &none)?;
    let tree = parse(out.as_bytes());
    let pad = " ".repeat(indent);
    let mut s = String::new();
    for m in wrapper_members(tree.root_node()) {
        if COMMENTS.contains(&m.kind()) { continue; }
        s.push_str(&pad);
        s.push_str(text(m, out.as_bytes()));
        s.push('\n');
    }
    Ok(s)
}

pub(super) fn equivalent(a: &str, b: &str, member: bool) -> bool {
    let (wa, wb) = (wrapped(a, member), wrapped(b, member));
    let (ta, tb) = (parse(wa.as_bytes()), parse(wb.as_bytes()));
    if first_error(ta.root_node()).is_some() || first_error(tb.root_node()).is_some() { return false; }
    let (mut x, mut y) = (vec![], vec![]);
    canon(ta.root_node(), wa.as_bytes(), &mut x);
    canon(tb.root_node(), wb.as_bytes(), &mut y);
    x == y
}
