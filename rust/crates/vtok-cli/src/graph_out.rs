//! `vtok graph report` and `vtok graph html`: a readable summary of the graph (hubs, subsystems, unexpected links,
//! questions worth asking) and a self-contained interactive page (no network, no libraries).
use crate::graph::Graph;
use std::collections::{BTreeMap, HashSet};

const MAX_HTML_NODES: usize = 700;

impl Graph {
    /// Edges between two different communities where neither end is a hub: the links a reader would not expect.
    fn surprising(&self, n: usize) -> Vec<(usize, usize)> {
        let deg = self.degree();
        let hub = (self.nodes.len() / 40).max(12);
        let mut v: Vec<(usize, usize)> = self.edges.iter().copied()
            .filter(|&(a, b)| self.label[a] != self.label[b] && deg[a] <= hub && deg[b] <= hub && self.nodes[a].is_code() && self.nodes[b].is_code() && self.nodes[a].kind != "value" && self.nodes[b].kind != "value" && self.nodes[a].file != self.nodes[b].file)
            .collect();
        v.sort_by_key(|&(a, b)| (std::cmp::Reverse(deg[a].min(deg[b])), a, b));
        v.truncate(n);
        v
    }

    pub fn report(&self) -> String {
        let mut out = format!("# Graph report\n\n{}\n", self.summary().trim());
        out.push_str("\n## Hubs (everything flows through these)\n");
        for l in self.hubs(10).lines().skip(1) { out.push_str(&format!("- {}\n", l.trim())); }
        out.push_str("\n## Subsystems (communities)\n");
        for l in self.communities_text(10).lines().skip(1) { out.push_str(&format!("- {l}\n")); }
        let sur = self.surprising(8);
        if !sur.is_empty() {
            out.push_str("\n## Unexpected links (different subsystems, neither end a hub)\n");
            for (a, b) in &sur { out.push_str(&format!("- {} -> {}  (C{} to C{})\n", self.short(*a), self.short(*b), self.label[*a], self.label[*b])); }
        }
        out.push_str("\n## Questions to ask the graph\n");
        let deg = self.degree();
        let mut idx: Vec<usize> = (0..self.nodes.len()).filter(|&i| deg[i] > 0 && self.nodes[i].is_code() && self.nodes[i].kind != "value").collect();
        idx.sort_by_key(|&i| std::cmp::Reverse(deg[i]));
        if let Some(&h) = idx.first() { out.push_str(&format!("- `vtok graph explain {}`: who depends on the most connected symbol?\n", crate::refs::base(&self.nodes[h].name))); }
        if idx.len() >= 2 { out.push_str(&format!("- `vtok graph path {} {}`: how do the two biggest hubs connect?\n", crate::refs::base(&self.nodes[idx[0]].name), crate::refs::base(&self.nodes[idx[1]].name))); }
        let mut docs: Vec<usize> = (0..self.nodes.len()).filter(|&i| deg[i] > 0 && !self.nodes[i].is_code() && self.nodes[i].kind != "page").collect();
        docs.sort_by_key(|&i| std::cmp::Reverse(deg[i]));
        if let Some(&d) = docs.first() { out.push_str(&format!("- `vtok graph explain {}`: which code does the best connected document ({}) cover?\n", self.nodes[d].label(), self.nodes[d].kind)); }
        if let Some((a, b)) = sur.first() { out.push_str(&format!("- `vtok graph path {} {}`: why do these two subsystems touch?\n", crate::refs::base(&self.nodes[*a].name), crate::refs::base(&self.nodes[*b].name))); }
        out
    }

    /// One self-contained page: the most connected symbols (capped), colored by community.
    pub fn html(&self) -> String {
        let deg = self.degree();
        let mut idx: Vec<usize> = (0..self.nodes.len()).collect();
        idx.sort_by_key(|&i| (std::cmp::Reverse(deg[i]), i));
        let cut = idx.len() > MAX_HTML_NODES;
        idx.truncate(MAX_HTML_NODES);
        let keep: HashSet<usize> = idx.iter().copied().collect();
        let remap: BTreeMap<usize, usize> = idx.iter().enumerate().map(|(n, &o)| (o, n)).collect();
        let mut names = BTreeMap::new();
        for &o in &idx { names.entry(self.label[o]).or_insert_with(|| self.community_name(self.label[o])); }
        let data = serde_json::json!({
            "nodes": idx.iter().map(|&o| { let n = &self.nodes[o]; serde_json::json!([n.file, n.name, n.a, n.b, n.kind]) }).collect::<Vec<_>>(),
            "edges": self.edges.iter().zip(&self.conf).filter(|(e, _)| keep.contains(&e.0) && keep.contains(&e.1)).map(|(e, c)| [remap[&e.0], remap[&e.1], *c as usize]).collect::<Vec<_>>(),
            "details": idx.iter().map(|&o| self.nodes[o].detail()).collect::<Vec<_>>(),
            "label": idx.iter().map(|&o| self.label[o]).collect::<Vec<_>>(),
            "names": names, "cut": cut, "total": self.nodes.len(),
        });
        include_str!("graph_view.html").replace("__DATA__", &data.to_string().replace("</", "<\\/"))
    }
}
