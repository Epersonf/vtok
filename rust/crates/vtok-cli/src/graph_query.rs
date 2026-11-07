//! Queries over a built graph: summary, hubs, communities, symbol lookup (explain and path are in graph_explain.rs).
use super::{Graph, INFERRED, RELS};
use std::collections::{BTreeMap, HashSet};

impl Graph {
    /// Degree over the semantic edges (contains/imports left out).
    pub(crate) fn degree(&self) -> Vec<usize> { let mut d = vec![0; self.nodes.len()]; for (a, b) in self.semantic() { d[a] += 1; d[b] += 1; } d }
    pub(crate) fn short(&self, i: usize) -> String {
        let n = &self.nodes[i];
        if n.is_code() { format!("{} {}:{}", n.name, n.file, n.a) } else if n.kind == "file" { n.name.clone() } else { format!("{} ({})", n.name, n.detail()) }
    }
    pub(crate) fn community_name(&self, c: usize) -> String {
        let deg = self.degree();
        let members: Vec<usize> = (0..self.nodes.len()).filter(|&i| self.label[i] == c).collect();
        let mut dirs: BTreeMap<String, usize> = BTreeMap::new();
        for &i in &members { let f = &self.nodes[i].file; *dirs.entry(f.rsplit_once('/').map_or(".", |x| x.0).to_string()).or_default() += 1; }
        let dir = dirs.into_iter().max_by_key(|(_, n)| *n).map(|x| x.0).unwrap_or_default();
        let top = members.iter().max_by_key(|&&i| deg[i]).map(|&i| self.nodes[i].label()).unwrap_or_default();
        format!("{dir} / {top}")
    }
    pub(crate) fn find(&self, q: &str) -> Vec<usize> {
        let by = |f: &dyn Fn(usize) -> bool| -> Vec<usize> { (0..self.nodes.len()).filter(|&i| f(i)).collect() };
        let exact = by(&|i| self.nodes[i].name == q);
        if !exact.is_empty() { return exact; }
        let tail = format!("/{q}");
        let files = by(&|i| !self.nodes[i].is_code() && (self.nodes[i].name.ends_with(&tail) || self.nodes[i].name.rsplit('#').last() == Some(q)));
        if !files.is_empty() { return files; }
        by(&|i| self.nodes[i].is_code() && crate::refs::base(&self.nodes[i].name) == q)
    }

    pub fn summary(&self) -> String {
        let ncomm = self.label.iter().max().map_or(0, |m| m + 1);
        let multi = (0..ncomm).filter(|&c| self.label.iter().filter(|&&l| l == c).count() > 1).count();
        let inferred = self.conf.iter().filter(|&&c| c == INFERRED).count();
        let mut kinds: BTreeMap<&str, usize> = BTreeMap::new();
        for n in &self.nodes { *kinds.entry(n.kind).or_default() += 1; }
        let ks: Vec<String> = kinds.iter().map(|(k, n)| format!("{k} {n}")).collect();
        let mut rels = [0usize; 16];
        for &r in &self.rel { rels[r as usize] += 1; }
        let rs: Vec<String> = RELS.iter().enumerate().filter(|(i, _)| rels[*i] > 0).map(|(i, r)| format!("{r} {}", rels[i])).collect();
        format!("graph: {} nodes ({} files), {} edges, {} communities with 2+ members\nedges: {} EXTRACTED, {inferred} INFERRED; {}\nnodes: {}\n",
            self.nodes.len(), self.files, self.edges.len(), multi, self.edges.len() - inferred, rs.join(", "), ks.join(", "))
    }

    pub fn hubs(&self, n: usize) -> String {
        let deg = self.degree();
        let mut idx: Vec<usize> = (0..self.nodes.len()).collect();
        idx.sort_by_key(|&i| std::cmp::Reverse(deg[i]));
        let mut out = String::from("# hubs: most connected symbols (callers + callees)\n");
        for &i in idx.iter().filter(|&&i| deg[i] > 0 && !matches!(self.nodes[i].kind, "value" | "file")).take(n) { out.push_str(&format!("{:>3} {} [{}]\n", deg[i], self.short(i), self.nodes[i].kind)); }
        out
    }

    pub fn communities_text(&self, n: usize) -> String {
        let ncomm = self.label.iter().max().map_or(0, |m| m + 1);
        let deg = self.degree();
        let mut out = String::from("# communities (tightly connected groups of symbols)\n");
        let mut shown = 0;
        for c in 0..ncomm {
            let mut m: Vec<usize> = (0..self.nodes.len()).filter(|&i| self.label[i] == c).collect();
            if m.len() < 2 { continue; }
            if shown >= n { break; }
            shown += 1;
            m.sort_by_key(|&i| std::cmp::Reverse(deg[i]));
            let files: HashSet<&str> = m.iter().map(|&i| self.nodes[i].file.as_str()).collect();
            out.push_str(&format!("C{c} {} — {} symbols in {} files: {}\n", self.community_name(c), m.len(), files.len(), m.iter().take(6).map(|&i| self.nodes[i].label()).collect::<Vec<_>>().join(", ")));
        }
        out
    }
}
