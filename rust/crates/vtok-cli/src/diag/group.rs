//! Grouping: identical diagnostics (code, message, elaboration) become one group with a list of places; groups of one
//! that differ only in a quoted name (`'f0' is defined but never used`) merge into a template group.
use super::quote::templates;
use super::D;
use std::collections::HashMap;

pub(super) struct Hit { pub file: String, pub line: u32, pub col: u32, pub arg: String }

pub(super) struct Group { pub warn: bool, pub code: String, pub msg: String, pub extra: Vec<String>, pub hits: Vec<Hit> }

/// Template groups need at least this many members to be worth a placeholder.
const MERGE_FROM: usize = 3;

pub(super) fn group(ds: &[D]) -> Vec<Group> {
    let mut seen = std::collections::HashSet::new();
    let mut idx: HashMap<(bool, &str, &str, &[String]), usize> = HashMap::new();
    let mut groups: Vec<Group> = vec![];
    for d in ds {
        if !seen.insert((&d.file, d.line, d.col, &d.code, &d.msg, &d.extra, d.warn)) { continue; }
        let hit = Hit { file: d.file.clone(), line: d.line, col: d.col, arg: String::new() };
        match idx.get(&(d.warn, d.code.as_str(), d.msg.as_str(), d.extra.as_slice())) {
            Some(&i) => groups[i].hits.push(hit),
            None => {
                idx.insert((d.warn, d.code.as_str(), d.msg.as_str(), d.extra.as_slice()), groups.len());
                groups.push(Group { warn: d.warn, code: d.code.clone(), msg: d.msg.clone(), extra: d.extra.clone(), hits: vec![hit] });
            }
        }
    }
    merge_templates(groups)
}

type Key = (bool, String, String, Vec<String>);

/// The masked versions of a singleton's message, as merge keys, with the text each one hides.
fn candidates(g: &Group) -> Vec<(Key, String)> {
    if g.hits.len() != 1 { return vec![]; }
    templates(&g.msg).into_iter().map(|(t, arg)| ((g.warn, g.code.clone(), t, g.extra.clone()), arg)).collect()
}

/// Each singleton joins the template it shares with the most others (the first one on ties); templates that end up
/// with fewer than `MERGE_FROM` members are dissolved, their members stay as they were.
fn merge_templates(groups: Vec<Group>) -> Vec<Group> {
    let cands: Vec<Vec<(Key, String)>> = groups.iter().map(candidates).collect();
    let mut shared: HashMap<&Key, usize> = HashMap::new();
    for (k, _) in cands.iter().flatten() { *shared.entry(k).or_default() += 1; }
    let mut members: HashMap<&Key, Vec<usize>> = HashMap::new();
    let mut choice: Vec<Option<&(Key, String)>> = vec![None; groups.len()];
    for (i, c) in cands.iter().enumerate() {
        for cand in c.iter().filter(|(k, _)| shared[k] >= MERGE_FROM) {
            if choice[i].map_or(true, |best| shared[&cand.0] > shared[&best.0]) { choice[i] = Some(cand); }
        }
        if let Some((k, _)) = choice[i] { members.entry(k).or_default().push(i); }
    }
    let mut out = vec![];
    let mut done: std::collections::HashSet<&Key> = std::collections::HashSet::new();
    for (i, g) in groups.iter().enumerate() {
        match choice[i].filter(|(k, _)| members[k].len() >= MERGE_FROM) {
            Some((k, _)) if done.insert(k) => {
                let hits = members[k].iter().map(|&m| {
                    let h = &groups[m].hits[0];
                    Hit { file: h.file.clone(), line: h.line, col: h.col, arg: choice[m].map(|c| c.1.clone()).unwrap_or_default() }
                }).collect();
                out.push(Group { warn: k.0, code: k.1.clone(), msg: k.2.clone(), extra: k.3.clone(), hits });
            }
            Some(_) => {}
            None => out.push(Group { warn: g.warn, code: g.code.clone(), msg: g.msg.clone(), extra: g.extra.clone(), hits: g.hits.iter().map(|h| Hit { file: h.file.clone(), line: h.line, col: h.col, arg: String::new() }).collect() }),
        }
    }
    out
}
