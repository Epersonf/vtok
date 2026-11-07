//! Which chunks a file has: by size (`plan_chunks`), by symbol (`plan_symbols`), or by the ids a caller asks for.
use crate::language::Language;
use crate::{Result, VtokError};
use std::collections::HashMap;

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
