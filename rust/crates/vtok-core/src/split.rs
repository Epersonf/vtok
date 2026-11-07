//! Split big files into addressable chunks; edit one chunk at a time; splice back into the ORIGINAL file.
use crate::aliasing::plan_aliases;
use crate::lang::{EncodeOpts, View};
use crate::language::{Analysis, Language};
use crate::{Result, VtokError};
use std::collections::{BTreeMap, HashMap};

#[derive(Clone, Debug)]
pub struct Chunk {
    pub id: String,
    pub label: String,
    pub start: usize,
    pub end: usize,
    pub member: bool,
    pub indent: usize,
}

impl Chunk {
    pub fn lines(&self) -> usize { self.end - self.start + 1 }
}

fn count_lines(s: &str) -> usize { s.lines().count() }

pub fn plan_chunks(lang: &dyn Language, source: &str, max_lines: usize) -> Result<Vec<Chunk>> {
    let n = count_lines(source);
    if n <= max_lines {
        return Ok(vec![Chunk { id: "c1".into(), label: "(whole file)".into(), start: 1, end: n, member: false, indent: 0 }]);
    }
    // [parent, start, end, names, indent, member]
    struct G { parent: Option<String>, start: usize, end: usize, names: Vec<String>, indent: usize, member: bool }
    let mut groups: Vec<G> = vec![];
    for u in lang.units(source, max_lines)? {
        if let Some(g) = groups.last_mut() {
            if g.parent == u.parent && (u.start <= g.end || u.end + 1 - g.start <= max_lines) {
                g.end = g.end.max(u.end);
                g.names.push(u.name.clone());
                continue;
            }
        }
        groups.push(G { parent: u.parent.clone(), start: u.start, end: u.end, names: vec![u.name.clone()], indent: u.indent, member: u.member });
    }
    let mut out = vec![];
    for (i, g) in groups.iter().enumerate() {
        let mut uniq: Vec<&String> = vec![];
        for n in &g.names { if !uniq.contains(&n) { uniq.push(n); } }
        let mut lab = uniq.iter().take(6).map(|s| s.as_str()).collect::<Vec<_>>().join(" ");
        if uniq.len() > 6 { lab.push_str(&format!(" +{}", uniq.len() - 6)); }
        let label = match &g.parent { Some(p) => format!("{p}: {lab}"), None => lab };
        out.push(Chunk { id: format!("c{}", i + 1), label, start: g.start, end: g.end, member: g.member, indent: g.indent });
    }
    Ok(out)
}

/// One stable alias table for many files, paid once.
pub fn plan_project(lang: &dyn Language, sources: &[&str], min_count: usize) -> BTreeMap<String, String> {
    let mut total = Analysis::default();
    for s in sources {
        if let Ok(a) = lang.analyze(s) {
            for (k, v) in a.counts { *total.counts.entry(k).or_insert(0) += v; }
            total.excluded.extend(a.excluded);
            total.avoid.extend(a.avoid);
            total.reserved.extend(a.reserved);
        }
    }
    plan_aliases(&total.counts, &total.avoid, &total.reserved, &total.excluded, min_count)
}

pub fn file_table(lang: &dyn Language, source: &str) -> BTreeMap<String, String> {
    plan_project(lang, &[source], 2)
}

pub fn chunk_source(source: &str, c: &Chunk) -> String {
    source.split_inclusive('\n').skip(c.start - 1).take(c.end - c.start + 1).collect()
}

/// Skeleton Vtok + chunk map: what the model reads first.
pub fn render_index(lang: &dyn Language, source: &str, chunks: &[Chunk], table: &BTreeMap<String, String>) -> Result<String> {
    let opts = EncodeOpts { view: View::Skeleton, table: Some(table.clone()), ..Default::default() };
    let mut sk = lang.encode(source, &opts)?.vtok;
    let mut rows = vec![];
    for c in chunks {
        let lab = c.label.replace(':', ": ").split_whitespace().map(|w| table.get(w).cloned().unwrap_or_else(|| w.to_string())).collect::<Vec<_>>().join(" ");
        rows.push(format!("{} L{}-{} {}", c.id, c.start, c.end, lab));
    }
    sk.push_str("\n# chunks (fetch one by id)\n");
    sk.push_str(&rows.join("\n"));
    sk.push('\n');
    Ok(sk)
}

pub fn chunk_vtok(lang: &dyn Language, source: &str, c: &Chunk, table: &BTreeMap<String, String>) -> Result<String> {
    lang.encode_chunk(&chunk_source(source, c), table, c.member)
}

/// edits: {chunk_id: vtok}. Returns the new source; unchanged chunks are not touched.
pub fn apply_edits(lang: &dyn Language, source: &str, chunks: &[Chunk], edits: &BTreeMap<String, String>,
                   table: &BTreeMap<String, String>, force: bool) -> Result<String> {
    let by_id: HashMap<&str, &Chunk> = chunks.iter().map(|c| (c.id.as_str(), c)).collect();
    let inv: HashMap<String, String> = table.iter().map(|(o, a)| (a.clone(), o.clone())).collect();
    let mut lines: Vec<String> = source.split_inclusive('\n').map(String::from).collect();
    let mut todo: Vec<(&Chunk, String)> = vec![];
    for (cid, vt) in edits {
        let Some(c) = by_id.get(cid.as_str()) else {
            let mut known: Vec<&str> = by_id.keys().copied().collect();
            known.sort();
            return Err(VtokError::new(format!("unknown chunk {cid:?}; known: {known:?}")));
        };
        let orig = chunk_source(source, c);
        let mut new = lang.decode_chunk(vt, &inv, c.member, c.indent, Some(&orig))?;
        if !force && lang.equivalent(&orig, &new, c.member) { continue; }
        let attempts = [patch_items(lang, &orig, vt, table, &inv, c), token_patch(lang, &orig, &new, c.member)];
        if let Some(p) = attempts.into_iter().flatten().find(|p| lang.equivalent(p, &new, c.member)) { new = p; }
        todo.push((c, new));
    }
    todo.sort_by(|a, b| b.0.start.cmp(&a.0.start));
    for w in todo.windows(2) {
        if w[1].0.end >= w[0].0.start { return Err(VtokError::new(format!("edits overlap: {} and {}", w[1].0.id, w[0].0.id))); }
    }
    for (c, new) in todo {
        let mut n = new;
        if !n.ends_with('\n') { n.push('\n'); }
        lines.splice(c.start - 1..c.end, [n]);
    }
    let out: String = lines.concat();
    lang.validate(&out)?;
    Ok(out)
}

/// Keep the original text (formatting, comments) of every top-level item whose Vtok line is unchanged:
/// only the changed run of items is replaced by decoded code. None = not applicable.
fn patch_items(lang: &dyn Language, orig: &str, vt: &str, table: &BTreeMap<String, String>,
               inv: &HashMap<String, String>, c: &Chunk) -> Option<String> {
    let spans = lang.item_spans(orig, c.member)?;
    let ov = lang.encode_chunk(orig, table, c.member).ok()?;
    let ol: Vec<&str> = ov.lines().filter(|l| !l.is_empty()).collect();
    let nl: Vec<&str> = vt.lines().filter(|l| !l.trim().is_empty()).collect();
    if ol.len() != spans.len() { return None; }
    let p = ol.iter().zip(&nl).take_while(|(a, b)| a == b).count();
    let s = ol[p..].iter().rev().zip(nl[p..].iter().rev()).take_while(|(a, b)| a == b).count();
    let (oe, ne) = (ol.len() - s, nl.len() - s);
    if p == ol.len() && p == nl.len() { return Some(orig.to_string()); }
    let pad = " ".repeat(c.indent);
    let new_mid = if ne > p {
        let d = lang.decode_chunk(&(nl[p..ne].join("\n") + "\n"), inv, c.member, c.indent, None).ok()?;
        d.strip_prefix(pad.as_str())?.strip_suffix('\n')?.to_string()
    } else { String::new() };
    let mut out = String::new();
    if oe > p {
        let (a, b) = (spans[p].0, spans[oe - 1].1);
        if new_mid.is_empty() {
            let a = orig[..a].rfind('\n').map_or(0, |i| i + 1);
            let b = if orig[b..].starts_with('\n') { b + 1 } else { b };
            out.push_str(&orig[..a]); out.push_str(&orig[b..]);
        } else {
            let region = &orig[a..b];
            let nm = token_patch(lang, region, &new_mid, c.member)
                .filter(|r| lang.equivalent(r, &new_mid, c.member)).unwrap_or(new_mid);
            out.push_str(&orig[..a]); out.push_str(&nm); out.push_str(&orig[b..]);
        }
    } else if p < spans.len() {
        let a = spans[p].0;
        out.push_str(&orig[..a]); out.push_str(&new_mid); out.push('\n'); out.push_str(&pad); out.push_str(&orig[a..]);
    } else {
        let b = spans.last()?.1;
        out.push_str(&orig[..b]); out.push('\n'); out.push_str(&pad); out.push_str(&new_mid); out.push_str(&orig[b..]);
    }
    Some(out)
}

/// One chunk per addressable symbol (function, method, type...), id = `name` or `Parent.name`
/// (`#2`, `#3` for repeated names). Lets a model ask for exactly the item it saw in the skeleton.
pub fn plan_symbols(lang: &dyn Language, source: &str) -> Result<Vec<Chunk>> {
    let mut seen: HashMap<String, usize> = HashMap::new();
    let mut out = vec![];
    for u in lang.units(source, 1)? {
        let base = match &u.parent { Some(p) => format!("{p}.{}", u.name), None => u.name.clone() };
        let n = seen.entry(base.clone()).or_insert(0);
        *n += 1;
        let id = if *n == 1 { base.clone() } else { format!("{base}#{n}") };
        out.push(Chunk { id, label: base, start: u.start, end: u.end, member: u.member, indent: u.indent });
    }
    Ok(out)
}

/// Chunks addressable by the given ids: the regular chunks plus symbol chunks when an id is not a chunk id.
/// A bare name that is unique as `Parent.name` resolves to it. Returns chunks with the ids rewritten to the
/// ids as requested.
pub fn resolve_chunks(lang: &dyn Language, source: &str, max_lines: usize, ids: &[&str]) -> Result<Vec<Chunk>> {
    let mut chunks = plan_chunks(lang, source, max_lines)?;
    if ids.iter().all(|i| chunks.iter().any(|c| c.id == *i)) { return Ok(chunks); }
    let syms = plan_symbols(lang, source)?;
    for want in ids {
        if chunks.iter().any(|c| c.id == *want) { continue; }
        let exact: Vec<&Chunk> = syms.iter().filter(|c| c.id == *want).collect();
        let found: Vec<&Chunk> = if !exact.is_empty() { exact } else {
            let suffix = format!(".{want}");
            syms.iter().filter(|c| c.id.ends_with(&suffix) || c.id.rsplit_once('.').map_or(false, |(_, n)| n == *want)).collect()
        };
        match found.len() {
            1 => { let mut c = found[0].clone(); c.id = want.to_string(); chunks.push(c); }
            0 => {
                let known: Vec<&str> = syms.iter().map(|c| c.id.as_str()).filter(|i| !i.starts_with("use") && !i.starts_with("imports") && !i.starts_with("import")).take(40).collect();
                return Err(VtokError::new(format!("unknown chunk or symbol {want:?}; symbols: {}", known.join(", "))));
            }
            _ => {
                let cands: Vec<&str> = found.iter().map(|c| c.id.as_str()).collect();
                return Err(VtokError::new(format!("symbol {want:?} is ambiguous; use one of: {}", cands.join(", "))));
            }
        }
    }
    Ok(chunks)
}

fn is_word(c: char) -> bool { c.is_alphanumeric() || c == '_' || c == '$' }

/// Token-level patch: turn `old` into `new` by keeping the original text (spacing, line breaks) of every
/// token that did not change and writing only the changed tokens. None = not applicable.
fn token_patch(lang: &dyn Language, old: &str, new: &str, member: bool) -> Option<String> {
    let ot = lang.tokens(old, member)?;
    let nt = lang.tokens(new, member)?;
    if ot.is_empty() || nt.is_empty() { return None; }
    let os: Vec<&str> = ot.iter().map(|&(a, b)| &old[a..b]).collect();
    let ns: Vec<&str> = nt.iter().map(|&(a, b)| &new[a..b]).collect();
    let pre = os.iter().zip(&ns).take_while(|(a, b)| a == b).count();
    let suf = os[pre..].iter().rev().zip(ns[pre..].iter().rev()).take_while(|(a, b)| a == b).count();
    let (om, nm) = (&os[pre..os.len() - suf], &ns[pre..ns.len() - suf]);
    if om.len().saturating_mul(nm.len()) > 4_000_000 { return None; }
    // LCS over the middle
    let (n, m) = (om.len(), nm.len());
    let w = m + 1;
    let mut t = vec![0u32; (n + 1) * w];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            t[i * w + j] = if om[i] == nm[j] { t[(i + 1) * w + j + 1] + 1 } else { t[(i + 1) * w + j].max(t[i * w + j + 1]) };
        }
    }
    // ops: (old index, new index); deletes before inserts inside a change region
    let mut ops: Vec<(Option<usize>, Option<usize>)> = (0..pre).map(|i| (Some(i), Some(i))).collect();
    let (mut i, mut j) = (0, 0);
    while i < n || j < m {
        if i < n && j < m && om[i] == nm[j] { ops.push((Some(pre + i), Some(pre + j))); i += 1; j += 1; }
        else if i < n && (j >= m || t[(i + 1) * w + j] >= t[i * w + j + 1]) { ops.push((Some(pre + i), None)); i += 1; }
        else { ops.push((None, Some(pre + j))); j += 1; }
    }
    for k in 0..suf { ops.push((Some(os.len() - suf + k), Some(ns.len() - suf + k))); }
    let gap = |text: &str, toks: &[(usize, usize)], i: usize| -> String {
        let s = if i == 0 { 0 } else { toks[i - 1].1 };
        text[s..toks[i].0].to_string()
    };
    let mut out = String::new(); // the first gap (taken from 0) already carries the leading whitespace
    let mut dels: std::collections::VecDeque<usize> = Default::default();
    for (o, nn) in ops {
        let (g, tok) = match (o, nn) {
            (Some(i), Some(_)) => { dels.clear(); (gap(old, &ot, i), os[i]) }
            (Some(i), None) => { dels.push_back(i); continue; }
            (None, Some(k)) => {
                let g = match dels.pop_front() {
                    Some(d) => { let g = gap(old, &ot, d); if g.chars().all(char::is_whitespace) { g } else { gap(new, &nt, k) } }
                    None => gap(new, &nt, k),
                };
                (g, ns[k])
            }
            _ => continue,
        };
        out.push_str(&g);
        if g.is_empty() && out.chars().last().map_or(false, is_word) && tok.chars().next().map_or(false, is_word) { out.push(' '); }
        out.push_str(tok);
    }
    out.push_str(&old[ot.last()?.1..]);
    Some(out)
}
