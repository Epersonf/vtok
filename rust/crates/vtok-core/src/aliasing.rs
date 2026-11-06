//! Exact break-even alias planning (same algorithm as the Python reference).
use crate::tokens::ntok;
use std::collections::{BTreeMap, HashMap, HashSet};

const TABLE_ROW_OVERHEAD: i64 = 2; // "=" and newline
static WORDS: &str = include_str!("../data_words.txt");

/// Cheapest names first: single letters, one-token words, letter+digit, two chars, then NkN forms.
pub fn alias_stream<'a>(avoid: &'a HashSet<String>) -> impl Iterator<Item = String> + 'a {
    let letters: Vec<char> = ('a'..='z').collect();
    let upper: Vec<char> = ('A'..='Z').collect();
    let digits: Vec<char> = ('0'..='9').collect();
    let mut all: Vec<String> = Vec::new();
    all.extend(letters.iter().map(|c| c.to_string()));
    all.extend(upper.iter().map(|c| c.to_string()));
    all.extend(WORDS.lines().filter(|w| !w.is_empty()).map(String::from));
    for l in &letters {
        for d in &digits {
            all.push(format!("{l}{d}"));
        }
    }
    for a in &letters {
        for b in letters.iter().chain(digits.iter()) {
            all.push(format!("{a}{b}"));
        }
    }
    let tail = (0usize..).flat_map(move |k| ('a'..='z').map(move |a| format!("{a}{k}{a}")));
    all.into_iter().chain(tail).filter(move |a| !avoid.contains(a))
}

/// counts: name -> occurrences. Returns {original: alias}.
pub fn plan_aliases(
    counts: &HashMap<String, usize>,
    avoid: &HashSet<String>,
    reserved: &HashSet<String>,
    excluded: &HashSet<String>,
    min_count: usize,
) -> BTreeMap<String, String> {
    let mut avoid_all = avoid.clone();
    avoid_all.extend(reserved.iter().cloned());
    let mut stream = alias_stream(&avoid_all);
    let mut forced: Vec<&String> = Vec::new();
    let mut cands: Vec<(i64, &String)> = Vec::new();
    for (name, &n) in counts {
        if reserved.contains(name) {
            // Reserved letters never serve as aliases (avoid_all), so an imported/exported
            // reserved name can safely stay as written; others are renamed away.
            if !excluded.contains(name) { forced.push(name); }
            continue;
        }
        if excluded.contains(name) || n < min_count {
            continue;
        }
        let l = ntok(name) as i64;
        let saving = n as i64 * (l - 1) - (l + 1 + TABLE_ROW_OVERHEAD);
        if saving > 0 {
            cands.push((saving, name));
        }
    }
    let mut table = BTreeMap::new();
    forced.sort();
    for name in forced {
        table.insert(name.clone(), stream.next().unwrap());
    }
    cands.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(b.1)));
    for (_, name) in cands {
        table.insert(name.clone(), stream.next().unwrap());
    }
    table
}
