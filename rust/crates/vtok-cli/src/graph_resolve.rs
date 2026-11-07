//! Symbol-level resolution: every identifier occurrence (graph_occ.rs) inside a symbol is resolved, by its syntax
//! position and the file's scope (graph_scope*.rs), to at most one definition, giving calls, uses_type and references
//! edges with a score. Bare names need scope evidence (same file 0.95, explicit import 0.9, glob import 0.85; a
//! non-strict language may use a unique global name 0.8 or narrow homonyms by directory 0.55-0.6); members resolve by
//! receiver (`self` 1.0, declared type 0.9, module alias 0.95, a unique uncommon method 0.85, type-like name 0.8,
//! trait dispatch 0.8 when the trait is in scope); paths by module or type qualifier. Scores follow the precision measured
//! per bucket by benchmarks/graph_eval.py. An ambiguous tie gives no edge. One definition in the whole
//! graph plus scope evidence is EXTRACTED (1.0).
use super::bind::{bindings, Bindings};
use super::edges::{enclosing, put_rel, EdgeMap, Parsed};
use super::occ::{occurrences, Form, Occ};
use super::scope::Scope;
use super::{Node, CALLS, REFERENCES, USES_TYPE};
use std::collections::HashMap;

#[path = "graph_resolve_member.rs"] mod member;

pub(crate) struct Ctx<'a> {
    pub nodes: &'a [Node],
    pub parsed: &'a [Parsed],
    pub sc: &'a [Scope],
    pub by_base: &'a HashMap<String, Vec<usize>>,
    /// parsed-file index of every node (usize::MAX for documents)
    pub fidx: Vec<usize>,
    pub trait_names: std::collections::HashSet<String>,
}

pub(crate) fn family(lang: &str) -> &str { if lang == "cpp" { "c" } else { lang } }

impl<'a> Ctx<'a> {
    pub(crate) fn new(nodes: &'a [Node], parsed: &'a [Parsed], sc: &'a [Scope], by_base: &'a HashMap<String, Vec<usize>>) -> Ctx<'a> {
        let mut fidx = vec![usize::MAX; nodes.len()];
        for (i, p) in parsed.iter().enumerate() { for &n in p.node_of.iter().flatten() { fidx[n] = i; } fidx[p.file_node] = i; }
        let trait_names = parsed.iter().flat_map(|p| p.traits.iter().map(|&t| crate::refs::base(&nodes[t].name).to_string())).collect();
        Ctx { nodes, parsed, sc, by_base, fidx, trait_names }
    }
    pub(crate) fn base(&self, i: usize) -> &str { crate::refs::base(&self.nodes[i].name) }
    /// Code nodes named `name` in the same language family, and (Rust) in a crate the file may use.
    pub(crate) fn named(&self, f: usize, name: &str, kinds: &[&str]) -> Vec<usize> {
        let fam = family(self.parsed[f].lang);
        self.by_base.get(name).map(|v| v.iter().copied().filter(|&c| {
            let cf = self.fidx[c];
            cf != usize::MAX && kinds.contains(&self.nodes[c].kind) && family(self.parsed[cf].lang) == fam
                && match (self.sc[f].krate, self.sc[cf].krate) { (Some(_), Some(k)) => self.sc[f].deps.contains(&k), _ => true }
        }).collect()).unwrap_or_default()
    }
    /// File `g` makes `name` from file `cf` visible to its importers: it defines it or re-exports it.
    pub(crate) fn exports(&self, g: usize, name: &str, cf: usize) -> bool {
        g == cf || self.sc[g].reexports.contains(&cf) || (self.sc[g].pub_names.contains(name) && self.sc[g].names.get(name).map_or(false, |(_, v)| v.contains(&cf)))
    }
    /// Scope evidence that file `f` sees `cand` under the local name `local`.
    pub(crate) fn tier(&self, f: usize, local: &str, cand: usize) -> Option<u8> {
        let cf = self.fidx[cand];
        if cf == f { return Some(95); }
        let s = &self.sc[f];
        if let Some((_, files)) = s.names.get(local) {
            if files.iter().any(|&x| self.exports(x, local, cf)) { return Some(90); }
            return None;
        }
        if s.globs.iter().any(|&g| self.exports(g, local, cf)) { return Some(85); }
        None
    }
    /// The one candidate with the best evidence; non-strict files may fall back to weaker evidence.
    pub(crate) fn pick(&self, f: usize, local: &str, cands: &[usize]) -> Option<(usize, u8)> {
        if cands.is_empty() { return None; }
        let tiers: Vec<Option<u8>> = cands.iter().map(|&c| self.tier(f, local, c)).collect();
        let best = tiers.iter().flatten().max().copied();
        if let Some(b) = best {
            let top: Vec<usize> = cands.iter().zip(&tiers).filter(|(_, t)| **t == Some(b)).map(|(c, _)| *c).collect();
            if top.len() != 1 { return None; }
            return Some((top[0], if cands.len() == 1 { 100 } else { b }));
        }
        let s = &self.sc[f];
        if s.strict || s.names.get(local).map_or(false, |x| x.1.is_empty()) { return None; }
        if cands.len() == 1 { return Some((cands[0], 80)); }
        let p = &self.parsed[f].rel;
        let dir = |c: usize| super::scope::dir_of(&self.nodes[c].file).to_string();
        let worded: Vec<usize> = cands.iter().copied().filter(|&c| { let n = &self.nodes[c].file; s.words.contains(super::scope::stem_of(n)) || s.words.contains(dir(c).rsplit('/').next().unwrap_or("")) }).collect();
        if worded.len() == 1 { return Some((worded[0], 60)); }
        let near: Vec<usize> = cands.iter().copied().filter(|&c| dir(c) == super::scope::dir_of(p)).collect();
        if near.len() == 1 { return Some((near[0], 55)); }
        None
    }

    fn bare(&self, f: usize, occ: &Occ, b: &Bindings) -> Option<(usize, u8, u8)> {
        let orig = self.sc[f].names.get(&occ.name).map_or(occ.name.as_str(), |x| x.0.as_str());
        let local = b.locals.contains(&occ.name);
        let kinds: &[&str] = if occ.head || (local && !occ.name.starts_with(|c: char| c.is_uppercase())) { &["type"] }
            else if local { &["type"] } else if occ.call { &["function", "type"] } else { &["type", "value", "function"] };
        let (c, s) = self.pick(f, &occ.name, &self.named(f, orig, kinds))?;
        let rel = match self.nodes[c].kind { "type" => USES_TYPE, "function" if occ.call => CALLS, _ => REFERENCES };
        Some((c, s, rel))
    }
}

pub(crate) fn resolve(nodes: &[Node], parsed: &[Parsed], sc: &[Scope], by_base: &HashMap<String, Vec<usize>>) -> EdgeMap {
    let cx = Ctx::new(nodes, parsed, sc, by_base);
    let mut m = EdgeMap::new();
    for (f, p) in parsed.iter().enumerate() {
        let lines: Vec<&str> = p.src.lines().collect();
        let mut binds: HashMap<usize, Bindings> = HashMap::new();
        let mut own_seen: std::collections::HashSet<usize> = Default::default();
        for occ in occurrences(p.lang, &p.src) {
            let Some(from) = enclosing(p, occ.line) else { continue };
            let n = &nodes[from];
            // the first mention of a symbol's own name near its top is its definition (`save() {` in a TS class)
            if occ.name == cx.base(from) && occ.line <= n.a + 12 && own_seen.insert(from) { continue; }
            let b = binds.entry(from).or_insert_with(|| bindings(p.lang, &lines[n.a.saturating_sub(1)..n.b.min(lines.len())].join("\n")));
            let hit = match occ.form {
                Form::Bare => cx.bare(f, &occ, b),
                Form::Member => cx.member(f, from, &occ, b).map(|(c, s)| (c, s, rel_for(&cx, c, occ.call))),
                Form::Path => cx.path(f, from, &occ).map(|(c, s)| (c, s, rel_for(&cx, c, occ.call))),
            };
            if let Some((to, score, rel)) = hit { put_rel(&mut m, from, to, rel, score); }
        }
    }
    m
}

fn rel_for(cx: &Ctx, c: usize, call: bool) -> u8 {
    match cx.nodes[c].kind { "type" => USES_TYPE, "function" | "method" if call => CALLS, _ => REFERENCES }
}
