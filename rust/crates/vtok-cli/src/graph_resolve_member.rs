//! Member (`x.f`) and path (`a::b::f`, `Type::f`, `Self::f`) resolution for graph_resolve.rs.
use super::super::bind::Bindings;
use super::super::names::common_method;
use super::super::occ::Occ;
use super::super::scope_rs::resolve_path;
use super::Ctx;

const FREE: &[&str] = &["function", "type", "value"];

fn snake(s: &str) -> String {
    let mut o = String::new();
    for (i, c) in s.chars().enumerate() { if c.is_uppercase() { if i > 0 { o.push('_'); } o.extend(c.to_lowercase()); } else { o.push(c); } }
    o
}

/// A receiver name that reads like the type: `repo` ~ Repo, `graph_store` ~ GraphStore, `cfg` ~ Config is not.
fn looks_like(recv: &str, owner: &str) -> bool {
    let (r, o) = (recv.to_ascii_lowercase(), owner.to_ascii_lowercase());
    !o.is_empty() && (r == o || recv == snake(owner) || (r.len() >= 4 && o.starts_with(&r)) || snake(owner).rsplit('_').next() == Some(recv))
}

impl<'a> Ctx<'a> {
    /// One candidate with owner `t` (directly, or the trait's declaration when `t` is a trait).
    fn by_owner(&self, cands: &[usize], t: &str) -> Option<usize> {
        let hit: Vec<usize> = cands.iter().copied().filter(|&c| self.nodes[c].owner() == t).collect();
        if hit.len() == 1 { Some(hit[0]) } else { None }
    }

    /// Items named `name` that module files `files` define or re-export (not methods).
    fn in_module(&self, f: usize, files: &[usize], name: &str) -> Option<(usize, u8)> {
        let all = self.named(f, name, FREE);
        let hit: Vec<usize> = all.iter().copied().filter(|&c| files.iter().any(|&m| self.exports(m, name, self.fidx[c]))).collect();
        if hit.len() == 1 { Some((hit[0], if all.len() == 1 { 100 } else { 95 })) } else { None }
    }

    pub(crate) fn member(&self, f: usize, from: usize, occ: &Occ, b: &Bindings) -> Option<(usize, u8)> {
        let recv = occ.recv.as_deref();
        if let Some(files) = recv.and_then(|r| self.sc[f].mods.get(r)).filter(|v| !v.is_empty()) {
            if let Some(hit) = self.in_module(f, files, &occ.name) { return Some(hit); }
        }
        if !occ.call && recv.map_or(true, |r| !r.starts_with(|c: char| c.is_uppercase())) { return None; }
        let cands = self.named(f, &occ.name, &["method", "value"]);
        let cands: Vec<usize> = cands.into_iter().filter(|&c| self.nodes[c].kind == "method" || !self.nodes[c].owner().is_empty()).collect();
        if cands.is_empty() { return None; }
        let uniq = |s: u8| if cands.len() == 1 { 100 } else { s };
        let own = self.nodes[from].owner();
        match recv {
            Some("self" | "this" | "Self" | "cls") if !own.is_empty() => {
                if let Some(c) = self.by_owner(&cands, own) { return Some((c, uniq(100))); }
                let t = self.nodes[from].impl_trait()?;
                return self.by_owner(&cands, t).map(|c| (c, 90));
            }
            Some(r) => {
                if let Some(t) = b.types.get(r) { if let Some(c) = self.by_owner(&cands, t) { return Some((c, uniq(90))); } }
                if r.starts_with(|c: char| c.is_uppercase()) && !self.named(f, r, &["type"]).is_empty() {
                    return self.by_owner(&cands, r).map(|c| (c, uniq(95)));
                }
                let like: Vec<usize> = cands.iter().copied().filter(|&c| looks_like(r, self.nodes[c].owner())).collect();
                if like.len() == 1 { return Some((like[0], 85)); }
            }
            None => {}
        }
        if !occ.call { return None; }
        // trait dispatch: every candidate is the trait's declaration or one of its impls
        let decl: Vec<usize> = cands.iter().copied().filter(|&c| self.trait_names.contains(self.nodes[c].owner())).collect();
        if decl.len() == 1 {
            let t = self.nodes[decl[0]].owner();
            if cands.iter().all(|&c| c == decl[0] || self.nodes[c].impl_trait() == Some(t)) { return Some((decl[0], 70)); }
        }
        if cands.len() == 1 && !common_method(&occ.name) { return Some((cands[0], 60)); }
        None
    }

    pub(crate) fn path(&self, f: usize, from: usize, occ: &Occ) -> Option<(usize, u8)> {
        let q = &occ.qual;
        let last = q.last()?.as_str();
        let assoc = || self.named(f, &occ.name, &["method", "value", "function"]).into_iter().filter(|&c| !self.nodes[c].owner().is_empty()).collect::<Vec<_>>();
        if last == "Self" {
            let own = self.nodes[from].owner();
            let cands = assoc();
            let c = self.by_owner(&cands, own).or_else(|| self.nodes[from].impl_trait().and_then(|t| self.by_owner(&cands, t)))?;
            return Some((c, if cands.len() == 1 { 100 } else { 95 }));
        }
        if self.parsed[f].lang == "rust" {
            let (file, n) = resolve_path(self.sc, f, q);
            if let (Some(m), true) = (file, n == q.len()) {
                // `crate::usage::record`: `usage` heads a path, so it is a module or a type, never the fn `usage`
                if occ.head && self.sc[m].children.contains_key(&occ.name) { return None; }
                return self.in_module(f, &[m], &occ.name).filter(|(c, _)| !occ.head || self.nodes[*c].kind == "type");
            }
            if n == 0 && q.len() > 1 { return None; }
        } else if let Some(files) = self.sc[f].mods.get(q[0].as_str()).filter(|_| q.len() == 1) {
            if let Some(hit) = self.in_module(f, files, &occ.name) { return Some(hit); }
        }
        // a type qualifier: the type must be visible here (`String::new` is not ours)
        let types = self.named(f, last, &["type"]);
        let (_, tscore) = self.pick(f, last, &types)?;
        let cands = assoc();
        let c = self.by_owner(&cands, last)?;
        Some((c, tscore))
    }
}
