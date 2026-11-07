//! Chunks: encode with a fixed alias table, decode with the docstrings of the original, and check that a chunk
//! round-trips.
use super::wrap::{body_of, wrapped};
use crate::canon::canon;
use crate::codec::{decode_raw, encode};
use crate::ctx::{Ctx, Line};
use crate::docs::{collect_docs, Docs};
use crate::emit::{emit_stmt, render};
use crate::pretty::pretty;
use crate::tree::{first_error, parse};
use crate::EncodeOpts;
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, HashSet};
use vtok_core::{Result, VtokError};

pub(super) fn encode_chunk(text: &str, table: &BTreeMap<String, String>, member: bool) -> Result<String> {
    let opts = EncodeOpts { table: Some(table.clone()), ..Default::default() };
    let vt = if !member {
        encode(text, &opts)?.vtok
    } else {
        let w = wrapped(text, true);
        let src = w.as_bytes();
        let tree = parse(src);
        if let Some(e) = first_error(tree.root_node()) {
            return Err(VtokError::at("source has syntax errors", e.start_position().row));
        }
        let empty: HashSet<String> = HashSet::new();
        let ctx = Ctx { ren: table, lean: false, skeleton: false, hide: 0, existing: &empty, lits: RefCell::new((BTreeMap::new(), HashMap::new(), 0)), src };
        let mut lines: Vec<Line> = vec![];
        for s in body_of(tree.root_node(), true) { emit_stmt(s, &ctx, 0, &mut lines); }
        render(lines)
    };
    // self-check: the chunk must decode back to something equivalent
    let inv: HashMap<String, String> = table.iter().map(|(o, a)| (a.clone(), o.clone())).collect();
    let pad = text.lines().find(|l| !l.trim().is_empty()).map(|l| l.len() - l.trim_start().len()).unwrap_or(0);
    let back = decode_chunk(&vt, &inv, member, if member { pad } else { 0 }, None)?;
    if !equivalent(text, &back, member) {
        return Err(VtokError::new("chunk roundtrip verification failed"));
    }
    Ok(vt)
}

pub(super) fn decode_chunk(vtok: &str, inv: &HashMap<String, String>, member: bool, indent: usize, original: Option<&str>) -> Result<String> {
    let raw = decode_raw(vtok, inv, &HashMap::new())?;
    let docs = match original {
        Some(o) => {
            let w = wrapped(o, member);
            let t = parse(w.as_bytes());
            if first_error(t.root_node()).is_some() { Docs::default() } else { collect_docs(&body_of(t.root_node(), member), w.as_bytes()) }
        }
        None => Docs::default(),
    };
    Ok(pretty(&raw, &docs, if member { indent } else { 0 }))
}

pub(super) fn equivalent(a: &str, b: &str, member: bool) -> bool {
    let (wa, wb) = (wrapped(a, member), wrapped(b, member));
    let (ta, tb) = (parse(wa.as_bytes()), parse(wb.as_bytes()));
    if first_error(ta.root_node()).is_some() || first_error(tb.root_node()).is_some() { return false; }
    let (mut x, mut y) = (vec![], vec![]);
    canon(ta.root_node(), wa.as_bytes(), true, &mut x);
    canon(tb.root_node(), wb.as_bytes(), true, &mut y);
    x == y
}
