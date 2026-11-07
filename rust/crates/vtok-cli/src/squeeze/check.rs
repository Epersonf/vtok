//! `--check`: what a rewrite must keep (fenced code, inline code, URLs, headings) and what is missing from it.

/// What a rewrite must keep: fenced code, inline code, URLs and headings. Returns what is missing from `new`.
pub fn missing(old: &str, new: &str) -> Vec<String> {
    let mut keep: Vec<String> = vec![];
    let mut fence = false;
    let mut block = String::new();
    for l in old.lines() {
        if l.trim_start().starts_with("```") { if fence { block.push_str(l.trim()); keep.push(std::mem::take(&mut block)); } fence = !fence; if fence { block.push_str(l.trim()); block.push('\n'); } continue; }
        if fence { block.push_str(l.trim_end()); block.push('\n'); continue; }
        if l.starts_with('#') { keep.push(l.trim().to_string()); }
        for (i, seg) in l.split('`').enumerate() { if i % 2 == 1 && !seg.is_empty() { keep.push(format!("`{seg}`")); } }
        for w in l.split_whitespace() { if let Some(p) = w.find("http") { let u = w[p..].trim_end_matches([')', ',', '.', ';', '>', '"']); if u.contains("://") { keep.push(u.to_string()); } } }
    }
    let squash = |t: &str| t.split_whitespace().collect::<Vec<_>>().join(" ");
    let hay = squash(new);
    keep.sort(); keep.dedup();
    keep.into_iter().filter(|k| !hay.contains(&squash(k))).collect()
}
