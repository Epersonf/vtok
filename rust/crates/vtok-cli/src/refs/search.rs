//! `vtok find WORDS`: symbols whose name words, parents or path match the words of a question.
use super::walk::walk;
use super::Report;
use std::path::Path;
use vtok_api::*;

const STOP: &[&str] = &["where", "is", "are", "the", "a", "an", "of", "to", "in", "on", "for", "and", "or", "how", "what", "which", "who", "does", "do", "handled", "handle", "handling", "code", "that", "this", "with", "from", "by", "used", "use", "find", "implemented", "implementation", "logic"];

/// `fooBar_baz2` -> ["foo", "bar", "baz", "2"], lowercased.
pub(super) fn name_tokens(s: &str) -> Vec<String> {
    let mut out: Vec<String> = vec![];
    let mut cur = String::new();
    let mut prev_lower = false;
    for c in s.chars() {
        if c.is_alphanumeric() {
            if c.is_uppercase() && prev_lower && !cur.is_empty() { out.push(std::mem::take(&mut cur)); }
            prev_lower = c.is_lowercase() || c.is_ascii_digit();
            cur.extend(c.to_lowercase());
        } else if !cur.is_empty() { out.push(std::mem::take(&mut cur)); prev_lower = false; }
    }
    if !cur.is_empty() { out.push(cur); }
    out
}

/// How well query word `w` matches name token `t`: exact 3, one is a prefix of the other (redirect/redirects, auth/authentication) 2.
pub(super) fn word_match(w: &str, t: &str) -> f64 {
    if w == t { 3.0 } else if w.chars().count() >= 4 && t.chars().count() >= 3 && (t.starts_with(w) || w.starts_with(t)) { 2.0 }
    else if w.chars().count() >= 5 && t.chars().count() >= 5 && w.chars().take(5).eq(t.chars().take(5)) { 1.5 } else { 0.0 }
}

/// Question-driven symbol search: the words of the question are matched against symbol names (split into words), their parents
/// and the file path. Returns ranked `file symbol L-range` lines ready for `vtok chunk`.
pub fn find_symbols(query: &str, root: &Path, budget: usize) -> Report {
    let words: Vec<String> = name_tokens(query).into_iter().filter(|w| !STOP.contains(&w.as_str()) && w.len() > 1).collect();
    let mut files = vec![];
    walk(root, &mut files);
    struct Cand { rel: String, id: String, a: usize, b: usize, name_t: Vec<String>, path_t: Vec<String>, sig_t: Vec<String> }
    let mut cands: Vec<Cand> = vec![];
    let mut raw_tokens = 0;
    for f in &files {
        let Ok(src) = std::fs::read_to_string(f) else { continue };
        let rel = f.strip_prefix(root).unwrap_or(f).display().to_string().replace(std::path::MAIN_SEPARATOR, "/");
        let Ok(l) = language_for_path(&rel) else { continue };
        let Ok(syms) = symbols(l.name(), &src) else { continue };
        raw_tokens += ntok(&src);
        let text: Vec<&str> = src.lines().collect();
        let path_t = name_tokens(&rel);
        for (id, a, b) in syms {
            if id == "use" || id.starts_with("use#") || id.starts_with("import") || id.starts_with("tests.") || id == "code" || id.starts_with("code#") { continue; }
            let sig = text.get(a - 1).map_or(String::new(), |t| t.trim().to_string());
            let id = id.split('#').next().unwrap_or(&id).to_string();
            cands.push(Cand { name_t: name_tokens(&id), sig_t: name_tokens(&sig), path_t: path_t.clone(), rel: rel.clone(), id, a, b });
        }
    }
    // rarer words weigh more: df = symbols whose name matches the word
    let n = cands.len().max(1) as f64;
    let weights: Vec<f64> = words.iter().map(|w| {
        let df = cands.iter().filter(|c| c.name_t.iter().any(|t| word_match(w, t) > 0.0)).count().max(1) as f64;
        1.0 + (n / df).ln().min(5.0)
    }).collect();
    let mut scored: Vec<(f64, &Cand)> = cands.iter().filter_map(|c| {
        let mut sc = 0.0;
        let mut hit_words = 0;
        for (w, wt) in words.iter().zip(&weights) {
            let by_name = c.name_t.iter().map(|t| word_match(w, t)).fold(0.0, f64::max);
            let by_path = c.path_t.iter().map(|t| word_match(w, t)).fold(0.0, f64::max) * 0.5;
            let by_sig = c.sig_t.iter().map(|t| word_match(w, t)).fold(0.0, f64::max) * 0.25;
            let m = by_name.max(by_path).max(by_sig);
            if m > 0.0 { hit_words += 1; sc += m * wt; }
        }
        // matching several words is worth more than one strong word
        (hit_words > 0).then(|| (sc * (1.0 + 0.5 * (hit_words as f64 - 1.0)), c))
    }).collect();
    scored.sort_by(|x, y| y.0.partial_cmp(&x.0).unwrap_or(std::cmp::Ordering::Equal).then_with(|| x.1.rel.cmp(&y.1.rel)).then(x.1.a.cmp(&y.1.a)));
    let mut out = format!("# find \"{}\": words {:?}; {} symbols matched (by name words, not by meaning)\n", query, words, scored.len());
    let mut shown = 0;
    for (_, c) in scored.iter().take(15) {
        let line = format!("{} {} {}\n", c.rel, c.id, if c.a == c.b { format!("L{}", c.a) } else { format!("L{}-{}", c.a, c.b) });
        if ntok(&(out.clone() + &line)) > budget { break; }
        out.push_str(&line);
        shown += 1;
    }
    if shown == 0 { out.push_str("# nothing matches: try other words (names are matched, not comments or meaning)\n"); }
    Report { text: out, raw_tokens, top: scored.iter().take(5).map(|(_, c)| (c.rel.clone(), c.id.clone())).collect() }
}
