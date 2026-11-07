//! Compact place lists: `a.ts:1,2,4,10:5,10:9` (the column only where two hits of a group share a line).
use super::group::{Group, Hit};

const WIDTH: usize = 110;

fn at(h: &Hit, all: &[&Hit]) -> String {
    let crowded = all.iter().filter(|o| o.line == h.line).count() > 1;
    let pos = if h.line == 0 { String::new() } else if crowded && h.col > 0 { format!("{}:{}", h.line, h.col) } else { h.line.to_string() };
    if h.arg.is_empty() { pos } else { format!("{pos} {}", h.arg) }
}

/// One chunk per file, in order of first appearance: `path:1,2,5`.
fn chunks(hits: &[&Hit]) -> Vec<String> {
    let mut files: Vec<(&str, Vec<&Hit>)> = vec![];
    for h in hits {
        match files.iter_mut().find(|f| f.0 == h.file) { Some(f) => f.1.push(h), None => files.push((&h.file, vec![h])) }
    }
    files.iter().map(|(f, hs)| {
        let items: Vec<String> = hs.iter().map(|h| at(h, hs)).collect();
        if hs.iter().all(|h| h.line == 0) { f.to_string() } else { format!("{f}:{}", items.join(",")) }
    }).collect()
}

/// The place lines of a group, at most `cap` places; a hit with a name (template groups) keeps it next to its line.
pub(super) fn lines(g: &Group, cap: usize) -> Vec<String> {
    // a cut that saves one or two places costs more than it saves (its own "... +1 more places" line)
    let cap = if g.hits.len() <= cap + 2 { g.hits.len() } else { cap };
    let shown: Vec<&Hit> = g.hits.iter().take(cap).collect();
    let mut out: Vec<String> = vec![];
    if shown.iter().any(|h| !h.arg.is_empty()) {
        out.extend(chunks(&shown));
    } else {
        for c in chunks(&shown) {
            match out.last_mut() { Some(l) if l.len() + c.len() < WIDTH => { l.push(' '); l.push_str(&c); } _ => out.push(c) }
        }
    }
    if g.hits.len() > cap { out.push(format!("... +{} more places", g.hits.len() - cap)); }
    out
}
