//! `explain` (a symbol's relations grouped by kind, scores shown when below 1) and `path` (shortest chain).
use super::{structural, tag, Graph, EXTRACTED, INFERRED, RELS};
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

/// How an incoming edge of each relation reads from the target's side.
const INCOMING: &[&str] = &["called by", "used as type by", "referenced by", "implemented by", "extended by", "in", "imported by", "tested by", "mentioned by"];
const MAX_LIST: usize = 12;

pub(crate) fn score_txt(s: u8) -> String { if s >= 100 { String::new() } else { format!(" {:.2}", s as f32 / 100.0) } }

impl Graph {
    pub fn explain(&self, q: &str) -> String {
        let hits = self.find(q);
        if hits.is_empty() { return format!("# no symbol named {q:?} in the graph\n"); }
        let mut out = String::new();
        for &i in hits.iter().take(4) {
            let n = &self.nodes[i];
            let place = if n.is_code() { format!("{}:{}-{}", n.file, n.a, n.b) } else { n.detail() };
            out.push_str(&format!("{} [{}] {}  community C{} ({})\n", n.name, n.kind, place, self.label[i], self.community_name(self.label[i])));
            // (outgoing?, relation) -> items
            let mut groups: BTreeMap<(bool, u8), Vec<(u8, String)>> = BTreeMap::new();
            for (e, &(a, b)) in self.edges.iter().enumerate() {
                let (out_edge, other) = if a == i { (true, b) } else if b == i { (false, a) } else { continue };
                groups.entry((!out_edge, self.rel[e])).or_default().push((self.score[e], self.short(other)));
            }
            for ((incoming, r), mut v) in groups {
                v.sort_by(|x, y| y.0.cmp(&x.0).then(x.1.cmp(&y.1)));
                let label = if incoming { INCOMING[r as usize] } else { RELS[r as usize] };
                let items: Vec<String> = v.iter().take(MAX_LIST).map(|(s, t)| format!("{t}{}", score_txt(*s))).collect();
                let more = if v.len() > MAX_LIST { format!(", … +{} more", v.len() - MAX_LIST) } else { String::new() };
                out.push_str(&format!("  {label} ({}): {}{more}\n", v.len(), items.join(", ")));
            }
        }
        out
    }

    pub fn path(&self, from: &str, to: &str) -> String {
        let (a, b) = (self.find(from), self.find(to));
        if a.is_empty() || b.is_empty() { return format!("# {} not found in the graph\n", if a.is_empty() { from } else { to }); }
        let goal: HashSet<usize> = b.iter().copied().collect();
        for directed in [true, false] {
            let mut adj: Vec<Vec<(usize, usize)>> = vec![vec![]; self.nodes.len()];
            for (e, &(x, y)) in self.edges.iter().enumerate() {
                if structural(self.rel[e]) { continue; }
                adj[x].push((y, e));
                if !directed { adj[y].push((x, e)); }
            }
            let mut prev: HashMap<usize, (usize, usize)> = HashMap::new();
            let mut seen: HashSet<usize> = a.iter().copied().collect();
            let mut q: VecDeque<usize> = a.iter().copied().collect();
            let mut hit = None;
            while let Some(v) = q.pop_front() {
                if goal.contains(&v) { hit = Some(v); break; }
                for &(w, e) in &adj[v] { if seen.insert(w) { prev.insert(w, (v, e)); q.push_back(w); } }
            }
            let Some(mut v) = hit else { continue };
            let mut chain = vec![(v, None)];
            while let Some(&(p, e)) = prev.get(&v) { chain.push((p, Some(e))); v = p; }
            chain.reverse();
            // a path is only as trustworthy as its weakest hop
            let weakest = chain.iter().filter_map(|c| c.1).map(|e| self.score[e]).min().unwrap_or(100);
            let mut line = String::new();
            for &(n, e) in &chain {
                line.push_str(&self.short(n));
                if let Some(e) = e {
                    let r = format!("{}{}", RELS[self.rel[e] as usize], score_txt(self.score[e]));
                    line.push_str(&if directed { format!(" -{r}-> ") } else { format!(" -{r}- ") });
                }
            }
            let c = if weakest >= 100 { EXTRACTED } else { INFERRED };
            return format!("# {} hops{}, confidence {}{}\n{}\n", chain.len() - 1, if directed { "" } else { " (ignoring direction)" }, tag(c), score_txt(weakest), line);
        }
        format!("# no connection between {from} and {to}\n")
    }
}
