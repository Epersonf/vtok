//! The public encode and decode entry points.
use crate::analysis::analyze;
use crate::canon::canon;
use crate::ctx::{Ctx, Line};
use crate::docs::Docs;
use crate::emit::{emit_stmt, render};
use crate::pretty::pretty;
use crate::tree::{body_statements, children, first_error, parse, text};
use crate::IMPORTS;
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, HashSet};
use vtok_core::aliasing::plan_aliases;
use vtok_core::lang::{EncodeOpts, Encoded, View};
use vtok_core::{Result, VtokError};

pub fn encode(source: &str, opts: &EncodeOpts) -> Result<Encoded> {
    let source = source.replace("\r\n", "\n");
    let src = source.as_bytes();
    let tree = parse(src);
    let root = tree.root_node();
    if let Some(e) = first_error(root) {
        return Err(VtokError::at("source has syntax errors", e.start_position().row + 1));
    }
    let an = analyze(root, src);
    let table: BTreeMap<String, String> = if let Some(t) = &opts.table {
        t.clone()
    } else if opts.alias {
        plan_aliases(&an.counts, &an.avoid, &HashSet::new(), &an.excluded, 2)
    } else {
        BTreeMap::new()
    };
    let mut existing: HashSet<String> = HashSet::new();
    if opts.hide_literals > 0 {
        let mut st = vec![root];
        while let Some(n) = st.pop() {
            if n.kind() == "string" { existing.insert(text(n, src).to_string()); }
            for c in children(n) { st.push(c); }
        }
    }
    let ctx = Ctx {
        ren: &table, lean: opts.view == View::Lean, skeleton: opts.view == View::Skeleton, hide: opts.hide_literals,
        existing: &existing, lits: RefCell::new((BTreeMap::new(), HashMap::new(), 0)), src,
    };
    let mut lines: Vec<Line> = vec![];
    for s in body_statements(root, true, src) { emit_stmt(s, &ctx, 0, &mut lines); }
    let vt = render(lines);
    let lits = ctx.lits.borrow().0.clone();
    if opts.verify && opts.view == View::Full {
        let inv: HashMap<String, String> = table.iter().map(|(o, a)| (a.clone(), o.clone())).collect();
        let back = decode(&vt, &inv, &lits.iter().map(|(k, v)| (k.clone(), v.clone())).collect())?;
        let bt = parse(back.as_bytes());
        let (mut a, mut b) = (vec![], vec![]);
        canon(bt.root_node(), back.as_bytes(), false, &mut a);
        canon(root, src, true, &mut b);
        if a != b { return Err(VtokError::new("roundtrip verification failed (tree differs)")); }
    } else if opts.verify {
        if let Some(e) = first_error(parse(vt.as_bytes()).root_node()) {
            return Err(VtokError::at("generated notation does not parse", e.start_position().row + 1));
        }
    }
    Ok(Encoded { vtok: vt, table, literals: lits })
}

/// table: {alias: original}; literals: {placeholder: original source text}.
pub fn decode_raw(vtok: &str, table: &HashMap<String, String>, literals: &HashMap<String, String>) -> Result<String> {
    let mut code = vtok.to_string();
    if !code.ends_with('\n') { code.push('\n'); }
    let tree = parse(code.as_bytes());
    let root = tree.root_node();
    if let Some(e) = first_error(root) {
        let lines: Vec<&str> = code.lines().collect();
        let row = e.start_position().row.min(lines.len().saturating_sub(1));
        let near: String = lines.get(row).map(|l| l.trim().chars().take(60).collect()).unwrap_or_default();
        return Err(VtokError::at(format!("syntax error near {near:?}"), row + 1));
    }
    if table.is_empty() && literals.is_empty() { return Ok(code); }
    let raw = code.as_bytes();
    let mut spans: Vec<(usize, usize, String)> = vec![];
    let mut st = vec![(root, false)];
    while let Some((n, in_imp)) = st.pop() {
        let k = n.kind();
        let imp = in_imp || IMPORTS.contains(&k);
        let s = text(n, raw);
        if k == "string" && literals.contains_key(s) {
            spans.push((n.start_byte(), n.end_byte(), literals[s].clone()));
            continue;
        }
        if k == "identifier" && !imp {
            if let Some(r) = table.get(s) { spans.push((n.start_byte(), n.end_byte(), r.clone())); }
        }
        for c in children(n) { st.push((c, imp)); }
    }
    spans.sort_by(|a, b| b.0.cmp(&a.0));
    let mut out = raw.to_vec();
    for (a, b, r) in spans { out.splice(a..b, r.into_bytes()); }
    Ok(String::from_utf8_lossy(&out).into_owned())
}

/// Reverse aliases and literals, then lay the statements out with ordinary 4-space indentation
/// (expressions stay compact). Valid Python either way.
pub fn decode(vtok: &str, table: &HashMap<String, String>, literals: &HashMap<String, String>) -> Result<String> {
    let raw = decode_raw(vtok, table, literals)?;
    Ok(pretty(&raw, &Docs::default(), 0))
}
