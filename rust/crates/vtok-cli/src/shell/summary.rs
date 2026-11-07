//! The generic summariser: collapse near-identical lines, then keep the lines that score highest within a token budget.
use super::clean::{cap, sig};
use super::score::score;
use std::collections::{BTreeSet, HashMap};
use vtok_api::ntok;

#[derive(Clone)]
pub(super) struct Group { pub first: usize, pub last: usize, pub text: String, pub count: usize }

pub(super) fn collapse(lines: &[String]) -> Vec<Group> {
    let mut gs: Vec<Group> = vec![];
    let mut last_sig = String::new();
    for (i, l) in lines.iter().enumerate() {
        let s = sig(l);
        if let Some(g) = gs.last_mut() {
            if !s.is_empty() && s == last_sig { g.last = i; g.count += 1; continue; }
        }
        last_sig = s;
        gs.push(Group { first: i, last: i, text: l.clone(), count: 1 });
    }
    dedupe_summaries(gs)
}

/// Key of a summary line: its shape plus every non-zero number, so `4 passed` and `7 passed` stay apart.
fn summary_key(text: &str) -> String {
    let mut k = sig(text);
    for w in text.split_whitespace() {
        let w = w.trim_end_matches(|c: char| !c.is_ascii_digit());
        if !w.is_empty() && w != "0" && w.bytes().all(|b| b.is_ascii_digit()) { k.push('|'); k.push_str(w); }
    }
    k
}

/// Non-adjacent summary lines that differ only in numbers (`test result: ok. 0 passed ...` once per crate) are
/// shown once with a total count; error lines are never merged.
fn dedupe_summaries(gs: Vec<Group>) -> Vec<Group> {
    let mut first: HashMap<String, usize> = HashMap::new();
    let mut out: Vec<Group> = vec![];
    for g in gs {
        if score(&g.text) == 2 {
            let k = summary_key(&g.text);
            if let Some(&i) = first.get(&k) { out[i].count += g.count; continue; }
            first.insert(k, out.len());
        }
        out.push(g);
    }
    out
}

pub(super) fn render(gs: &[Group], sel: &BTreeSet<usize>) -> String {
    let mut out = String::new();
    let mut prev: Option<usize> = None;
    for &i in sel {
        match prev {
            Some(p) if p + 1 == i => {}
            Some(p) => out.push_str(&format!("… {} lines omitted …\nL{}:\n", gs[i].first - gs[p].last - 1, gs[i].first + 1)),
            None if gs[i].first > 0 => out.push_str(&format!("L{}:\n", gs[i].first + 1)),
            None => {}
        }
        let g = &gs[i];
        if g.text.is_empty() { prev = Some(i); continue; }
        out.push_str(&cap(&g.text));
        if g.count > 1 { out.push_str(&format!("  [×{} similar]", g.count)); }
        out.push('\n');
        prev = Some(i);
    }
    if let Some(p) = prev {
        let total = gs.last().map_or(0, |g| g.last);
        if gs[p].last < total { out.push_str(&format!("… {} lines omitted …\n", total - gs[p].last)); }
    }
    out
}

/// Compact view of `lines` that fits `budget` tokens.
pub(super) fn summarize(lines: &[String], budget: usize) -> String {
    let gs = collapse(lines);
    let n = gs.len();
    let scores: Vec<u8> = gs.iter().map(|g| score(&g.text)).collect();
    for (head, tail, ctx) in [(0usize, 1usize, 1usize), (0, 1, 0)] {
        let mut sel: BTreeSet<usize> = BTreeSet::new();
        for i in 0..head.min(n) { sel.insert(i); }
        for i in n.saturating_sub(tail)..n { sel.insert(i); }
        let mut by_score: Vec<usize> = (0..n).filter(|&i| scores[i] > 0).collect();
        by_score.sort_by_key(|&i| (std::cmp::Reverse(scores[i]), i));
        let mut best = render(&gs, &sel);
        for i in by_score {
            let mut trial = sel.clone();
            let c = if scores[i] >= 3 { ctx } else { 0 };
            for j in i.saturating_sub(c)..(i + c + 1).min(n) { if j == i || !(gs[j].count > 1 && scores[j] == 0) { trial.insert(j); } }
            let r = render(&gs, &trial);
            if ntok(&r) > budget { if scores[i] >= 3 { continue; } else { break; } }
            sel = trial;
            best = r;
        }
        if ntok(&best) <= budget { return best; }
    }
    render(&gs, &(n.saturating_sub(1)..n).collect())
}
