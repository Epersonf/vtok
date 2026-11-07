//! `vtok output [ID]`: read part of a stored run back (grep, line range, head, tail). Without an id: the most recent run.
use super::clean::cap;
use super::store::{latest, load};
use std::collections::BTreeSet;
use vtok_api::ntok;

const GREP_LINES: usize = 40;

pub struct Query { pub grep: Vec<String>, pub range: Option<(usize, usize)>, pub tail: Option<usize>, pub head: Option<usize>, pub context: usize, pub max_tokens: usize }

impl Query {
    fn is_empty(&self) -> bool { self.grep.is_empty() && self.range.is_none() && self.tail.is_none() && self.head.is_none() }
}

/// The selected parts of stored output `id` (the most recent run when `id` is None), as numbered lines.
pub fn fetch(id: Option<&str>, q: &Query) -> Result<String, String> {
    let id = match id { Some(i) => i.to_string(), None => latest().ok_or("no stored output yet (run a command with `vtok run -- ...` first)")? };
    Ok(select(&id, &load(&id)?, q))
}

pub(super) fn select(id: &str, lines: &[String], q: &Query) -> String {
    let n = lines.len();
    let mut sel: BTreeSet<usize> = BTreeSet::new();
    let mut grep_total = 0;
    if !q.grep.is_empty() {
        let needles: Vec<String> = q.grep.iter().map(|s| s.to_lowercase()).collect();
        for (i, l) in lines.iter().enumerate() {
            let low = l.to_lowercase();
            if needles.iter().any(|k| low.contains(k.as_str())) { for j in i.saturating_sub(q.context)..(i + q.context + 1).min(n) { sel.insert(j); } }
        }
        grep_total = sel.len();
        if grep_total > GREP_LINES { sel = sel.into_iter().take(GREP_LINES).collect(); }
    }
    if let Some((a, b)) = q.range { for j in a.max(1) - 1..b.min(n) { sel.insert(j); } }
    if let Some(t) = q.tail { for j in n.saturating_sub(t)..n { sel.insert(j); } }
    if let Some(h) = q.head { for j in 0..h.min(n) { sel.insert(j); } }
    if sel.is_empty() && q.is_empty() { for j in n.saturating_sub(40)..n { sel.insert(j); } }
    let mut out = format!("# output {id}: {n} lines\n");
    let mut prev: Option<usize> = None;
    let mut used = ntok(&out);
    for j in sel {
        if let Some(p) = prev { if p + 1 != j { out.push_str("--\n"); } }
        let line = format!("{}: {}\n", j + 1, cap(&lines[j]));
        used += ntok(&line);
        if used > q.max_tokens { out.push_str(&format!("# truncated at ~{} tokens; narrow with --grep / --lines\n", q.max_tokens)); break; }
        out.push_str(&line);
        prev = Some(j);
    }
    if grep_total > GREP_LINES { out.push_str(&format!("# grep: showing the first {GREP_LINES} of {grep_total} lines; narrow the pattern or use --lines\n")); }
    if prev.is_none() { out.push_str("# no matching lines\n"); }
    out
}

pub fn parse_range(s: &str) -> Option<(usize, usize)> {
    let (a, b) = s.split_once('-').unwrap_or((s, s));
    Some((a.trim().parse().ok()?, b.trim().parse().ok()?))
}

const USAGE: &str = "usage: vtok output [ID] [--grep A|B] [--lines A-B] [--tail N] [--head N] [--context N] [--max-tokens N]   (no ID: the most recent run)";

pub fn output_cmd(args: &[String]) -> i32 {
    let mut id = None;
    let mut q = Query { grep: vec![], range: None, tail: None, head: None, context: 2, max_tokens: 2000 };
    let mut i = 0;
    while i < args.len() {
        let next = |i: &mut usize| -> Option<String> { *i += 1; args.get(*i).cloned() };
        match args[i].as_str() {
            "--grep" => { if let Some(v) = next(&mut i) { q.grep.extend(v.split('|').map(String::from)); } }
            "--lines" => { q.range = next(&mut i).and_then(|v| parse_range(&v)); }
            "--tail" => { q.tail = next(&mut i).and_then(|v| v.parse().ok()); }
            "--head" => { q.head = next(&mut i).and_then(|v| v.parse().ok()); }
            "--context" => { q.context = next(&mut i).and_then(|v| v.parse().ok()).unwrap_or(2); }
            "--max-tokens" => { q.max_tokens = next(&mut i).and_then(|v| v.parse().ok()).unwrap_or(2000); }
            "--help" | "-h" => { println!("{USAGE}"); return 0; }
            a if !a.starts_with("--") && id.is_none() => id = Some(a.to_string()),
            a => { eprintln!("unknown argument {a:?}\n{USAGE}"); return 2; }
        }
        i += 1;
    }
    match fetch(id.as_deref(), &q) { Ok(s) => { print!("{s}"); 0 } Err(e) => { eprintln!("error: {e}"); 1 } }
}
