//! The public encode and decode entry points.
use crate::analysis::{analyze, words};
use crate::canon::canon;
use crate::ctx::Ctx;
use crate::lines::native_lines;
use crate::literals::collect_literals;
use crate::tree::{children, first_error, named_children, parse, text};
use crate::{is_ident, RESERVED_WORDS};
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
        return Err(VtokError::new(format!("source has syntax errors near line {}", e.start_position().row + 1)));
    }
    let an = analyze(root, src);
    let reserved: HashSet<String> = words(RESERVED_WORDS).collect();
    let table: BTreeMap<String, String> = if let Some(t) = &opts.table {
        t.clone()
    } else if opts.alias {
        plan_aliases(&an.counts, &an.avoid, &reserved, &an.excl, 2)
    } else {
        let bad: HashMap<String, usize> = an.counts.keys().filter(|n| reserved.contains(*n)).map(|n| (n.clone(), 1)).collect();
        if bad.is_empty() { BTreeMap::new() } else { plan_aliases(&bad, &HashSet::new(), &reserved, &HashSet::new(), 2) }
    };
    let (pos, lits) = if opts.hide_literals > 0 { collect_literals(root, src, opts.hide_literals) } else { (HashMap::new(), BTreeMap::new()) };
    let ctx = Ctx { ren: &table, skeleton: opts.view == View::Skeleton, lean: opts.view == View::Lean, lits: &pos, src };
    let top = named_children(root);
    let mut vt = native_lines(&top, &ctx)?.join("\n");
    vt.push('\n');
    if opts.verify && opts.view != View::Lean {
        let inv: HashMap<String, String> = table.iter().map(|(o, a)| (a.clone(), o.clone())).collect();
        let back = decode(&vt, &inv, &lits.iter().map(|(k, v)| (k.clone(), v.clone())).collect())?;
        if opts.view == View::Full {
            let bt = parse(back.as_bytes());
            let (mut a, mut b) = (vec![], vec![]);
            canon(bt.root_node(), back.as_bytes(), &mut a);
            canon(root, src, &mut b);
            if a != b { return Err(VtokError::new("roundtrip verification failed (tree differs)")); }
        }
    }
    Ok(Encoded { vtok: vt, table, literals: lits })
}

/// table: {alias: original}; literals: {placeholder: original text}.
pub fn decode(vtok: &str, table: &HashMap<String, String>, literals: &HashMap<String, String>) -> Result<String> {
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
    let mut st = vec![root];
    while let Some(n) = st.pop() {
        for c in children(n) { st.push(c); }
        let t = n.kind();
        let s = text(n, raw);
        if t == "string" && literals.contains_key(s) {
            spans.push((n.start_byte(), n.end_byte(), literals[s].clone()));
        } else if is_ident(t) && table.contains_key(s) {
            spans.push((n.start_byte(), n.end_byte(), table[s].clone()));
        }
    }
    spans.sort_by(|a, b| b.0.cmp(&a.0));
    let mut out = raw.to_vec();
    for (a, b, r) in spans { out.splice(a..b, r.into_bytes()); }
    Ok(String::from_utf8_lossy(&out).into_owned())
}
