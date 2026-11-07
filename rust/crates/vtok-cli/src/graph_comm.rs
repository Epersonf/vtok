//! Community detection for `vtok graph`: deterministic label propagation over the undirected graph.
use super::Node;
use std::collections::{BTreeMap, HashMap};

/// Labels are renumbered by community size (0 is the largest).
pub(crate) fn communities(nodes: &[Node], edges: &[(usize, usize)]) -> Vec<usize> {
    let n = nodes.len();
    // plain values (locals, constants, parameters) are glue, not structure: they never join a group
    let edges: Vec<(usize, usize)> = edges.iter().copied().filter(|&(a, b)| !matches!(nodes[a].kind, "value" | "file") && !matches!(nodes[b].kind, "value" | "file")).collect();
    let edges = &edges[..];
    // god nodes (shared types and helpers used everywhere) glue unrelated code together: leave them out of the grouping
    let mut deg = vec![0usize; n];
    for &(a, b) in edges { deg[a] += 1; deg[b] += 1; }
    let cap = (n / 12).max(25);
    let mut adj: Vec<Vec<usize>> = vec![vec![]; n];
    for &(a, b) in edges { if deg[a] <= cap && deg[b] <= cap { adj[a].push(b); adj[b].push(a); } }
    let mut lab: Vec<usize> = (0..n).collect();
    for _ in 0..12 {
        let mut changed = false;
        for v in 0..n {
            if adj[v].is_empty() { continue; }
            let mut cnt: BTreeMap<usize, usize> = BTreeMap::new();
            for &u in &adj[v] { *cnt.entry(lab[u]).or_default() += 1; }
            let best = cnt.iter().max_by(|a, b| a.1.cmp(b.1).then(b.0.cmp(a.0))).map(|(l, _)| *l).unwrap_or(lab[v]);
            if best != lab[v] { lab[v] = best; changed = true; }
        }
        if !changed { break; }
    }
    let mut size: HashMap<usize, usize> = HashMap::new();
    for &l in &lab { *size.entry(l).or_default() += 1; }
    let mut order: Vec<(usize, usize)> = size.into_iter().collect();
    order.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let re: HashMap<usize, usize> = order.iter().enumerate().map(|(i, (l, _))| (*l, i)).collect();
    lab.iter().map(|l| re[l]).collect()
}
