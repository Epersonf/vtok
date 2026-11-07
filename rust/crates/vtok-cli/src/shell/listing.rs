//! Listings (find, grep -rn, ls -R) are structure, not prose: group by directory, as deep as the budget allows.
use super::clean::cap;
use std::collections::HashMap;
use vtok_api::ntok;

/// `path:line:text` (grep -n / rg) -> (path, text)
fn grep_hit(l: &str) -> Option<(&str, &str)> {
    let (path, rest) = l.split_once(':')?;
    let (num, text) = rest.split_once(':')?;
    (!path.is_empty() && (path.contains('/') || path.contains('.')) && !path.contains(' ') && !num.is_empty() && num.bytes().all(|b| b.is_ascii_digit())).then_some((path, text))
}

fn is_path_line(l: &str) -> bool { !l.trim().is_empty() && !l.contains(' ') && l.contains('/') && !l.contains(':') }

fn parts_of(p: &str) -> Vec<&str> { p.split('/').filter(|c| !c.is_empty() && *c != ".").collect() }

/// The directory prefix of `p` at depth `d` (a file sitting higher up keeps its own directory).
fn key_at(p: &str, d: usize) -> String {
    let parts = parts_of(p);
    let lead = if p.starts_with('/') { "/" } else { "" };
    let cut = if parts.len() <= d { parts.len() - 1 } else { d };
    let dir = parts[..cut].join("/");
    if dir.is_empty() { format!("{lead}.") } else { format!("{lead}{dir}/") }
}

/// Too long even at the coarsest grouping: keep the biggest groups.
fn biggest_groups(out: &str, budget: usize) -> String {
    let mut ls: Vec<&str> = out.lines().collect();
    let total = ls.len();
    ls.sort_by_key(|l| std::cmp::Reverse(l.split_whitespace().nth(1).and_then(|n| n.parse::<usize>().ok()).unwrap_or(0)));
    let mut keep = String::new();
    let mut shown = 0;
    for l in ls { if ntok(&keep) + ntok(l) > budget { break; } keep.push_str(l); keep.push('\n'); shown += 1; }
    if shown < total { keep.push_str(&format!("... {} more groups\n", total - shown)); }
    keep
}

pub(super) fn summarize_listing(lines: &[String], budget: usize) -> Option<String> {
    let rows: Vec<&String> = lines.iter().filter(|l| !l.trim().is_empty()).collect();
    if rows.len() < 20 { return None; }
    let greps = rows.iter().filter(|l| grep_hit(l).is_some()).count();
    let paths = rows.iter().filter(|l| is_path_line(l)).count();
    let hits = greps * 10 >= rows.len() * 8;
    if !hits && paths * 10 < rows.len() * 8 { return None; }
    let items: Vec<(&str, &str)> = rows.iter().filter_map(|l| if hits { grep_hit(l) } else { is_path_line(l).then_some((l.as_str(), "")) }).collect();
    let max_d = items.iter().map(|(p, _)| parts_of(p).len()).max().unwrap_or(1);
    let first = parts_of(items[0].0);
    let mut common = first.len().saturating_sub(1);
    for (p, _) in &items { let q = parts_of(p); common = common.min(first.iter().zip(&q).take_while(|(a, b)| a == b).count()).min(q.len().saturating_sub(1)); }
    let mut best = None;
    for d in (common + 1)..=max_d.max(common + 1) {
        let mut groups: Vec<(String, usize, &str)> = vec![];
        let mut idx: HashMap<String, usize> = HashMap::new();
        for (p, t) in &items {
            let k = key_at(p, d);
            match idx.get(&k) {
                Some(&i) => groups[i].1 += 1,
                None => { idx.insert(k.clone(), groups.len()); groups.push((k, 1, t)); }
            }
        }
        let mut out = String::new();
        for (k, n, t) in &groups {
            out.push_str(&format!("{k}  {n} {}", if hits { "matches" } else { "files" }));
            if hits { out.push_str(&format!("   e.g. {}", cap(t.trim()))); }
            out.push('\n');
        }
        if ntok(&out) > budget {
            if best.is_none() { best = Some(biggest_groups(&out, budget)); }
            break;
        }
        best = Some(out);
    }
    let kind = if hits { "grep matches" } else { "paths" };
    best.map(|b| format!("{} {kind} in total, grouped by directory\n{b}", items.len()))
}
