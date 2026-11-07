//! Queries over a built graph: summary, hubs, communities, explain, path.
use super::{tag, Graph, INFERRED};
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

impl Graph {
    pub(crate) fn degree(&self) -> Vec<usize> { let mut d = vec![0; self.nodes.len()]; for &(a, b) in &self.edges { d[a] += 1; d[b] += 1; } d }
    pub(crate) fn short(&self, i: usize) -> String {
        let n = &self.nodes[i];
        if n.is_code() { format!("{} {}:{}", n.name, n.file, n.a) } else { format!("{} ({})", n.name, n.detail()) }
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
    fn find(&self, q: &str) -> Vec<usize> {
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
        format!("graph: {} nodes ({} files), {} edges, {} communities with 2+ members\nedges: {} EXTRACTED, {inferred} INFERRED; nodes: {}\n",
            self.nodes.len(), self.files, self.edges.len(), multi, self.edges.len() - inferred, ks.join(", "))
    }

    pub fn hubs(&self, n: usize) -> String {
        let deg = self.degree();
        let mut idx: Vec<usize> = (0..self.nodes.len()).collect();
        idx.sort_by_key(|&i| std::cmp::Reverse(deg[i]));
        let mut out = String::from("# hubs: most connected symbols (callers + callees)\n");
        for &i in idx.iter().filter(|&&i| deg[i] > 0 && self.nodes[i].kind != "value").take(n) { out.push_str(&format!("{:>3} {} [{}]\n", deg[i], self.short(i), self.nodes[i].kind)); }
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

    pub fn explain(&self, q: &str) -> String {
        let hits = self.find(q);
        if hits.is_empty() { return format!("# no symbol named {q:?} in the graph\n"); }
        let mut out = String::new();
        for &i in hits.iter().take(4) {
            let n = &self.nodes[i];
            let place = if n.is_code() { format!("{}:{}-{}", n.file, n.a, n.b) } else { n.detail() };
            out.push_str(&format!("{} [{}] {}  community C{} ({})\n", n.name, n.kind, place, self.label[i], self.community_name(self.label[i])));
            let side = |from: bool| -> Vec<String> { (0..self.edges.len()).filter(|&e| (if from { self.edges[e].1 } else { self.edges[e].0 }) == i).map(|e| format!("{} [{}]", self.short(if from { self.edges[e].0 } else { self.edges[e].1 }), tag(self.conf[e]))).collect() };
            let (callers, callees) = (side(true), side(false));
            let lst = |v: &Vec<String>| if v.is_empty() { "-".to_string() } else if v.len() > 12 { format!("{}, … +{} more", v[..12].join(", "), v.len() - 12) } else { v.join(", ") };
            out.push_str(&format!("  used by ({}): {}\n  uses ({}): {}\n", callers.len(), lst(&callers), callees.len(), lst(&callees)));
        }
        out
    }

    pub fn path(&self, from: &str, to: &str) -> String {
        let (a, b) = (self.find(from), self.find(to));
        if a.is_empty() || b.is_empty() { return format!("# {} not found in the graph\n", if a.is_empty() { from } else { to }); }
        let goal: HashSet<usize> = b.iter().copied().collect();
        for directed in [true, false] {
            let mut adj: Vec<Vec<(usize, usize)>> = vec![vec![]; self.nodes.len()];
            for (e, &(x, y)) in self.edges.iter().enumerate() { adj[x].push((y, e)); if !directed { adj[y].push((x, e)); } }
            let mut prev: HashMap<usize, (usize, usize)> = HashMap::new();
            let mut seen: HashSet<usize> = a.iter().copied().collect();
            let mut q: VecDeque<usize> = a.iter().copied().collect();
            let mut hit = None;
            while let Some(v) = q.pop_front() {
                if goal.contains(&v) { hit = Some(v); break; }
                for &(w, e) in &adj[v] { if seen.insert(w) { prev.insert(w, (v, e)); q.push_back(w); } }
            }
            if let Some(mut v) = hit {
                let mut chain = vec![(v, None)];
                while let Some(&(p, e)) = prev.get(&v) { chain.push((p, Some(e))); v = p; }
                chain.reverse();
                // a path is only as trustworthy as its weakest hop
                let weakest = chain.iter().filter_map(|c| c.1).map(|e| self.conf[e]).max().unwrap_or(0);
                let mut line = String::new();
                for (k, &(n, e)) in chain.iter().enumerate() {
                    line.push_str(&self.short(n));
                    if k + 1 < chain.len() { line.push_str(&format!(" {} ", match (e.map(|e| self.conf[e]), directed) { (Some(INFERRED), true) => "~>", (Some(INFERRED), false) => "~~", (_, true) => "->", _ => "--" })); }
                }
                return format!("# {} hops{}, confidence {}\n{}\n", chain.len() - 1, if directed { "" } else { " (ignoring call direction)" }, tag(weakest), line);
            }
        }
        format!("# no connection between {from} and {to}\n")
    }
}
