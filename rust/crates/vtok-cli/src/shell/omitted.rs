//! Does a view admit that it left something out? Views drop chatter silently but say so when they drop content
//! ("+3 more", "... 12 lines omitted", "[x4 similar]"); `vtok run` then tells the agent where the rest is stored.

/// "+12 more", "+3 more files": a plus sign, digits, the word "more".
fn plus_n_more(low: &str) -> bool {
    let b = low.as_bytes();
    (0..b.len()).any(|i| {
        if b[i] != b'+' { return false; }
        let d = b[i + 1..].iter().take_while(|c| c.is_ascii_digit()).count();
        d > 0 && low[i + 1 + d..].trim_start().starts_with("more")
    })
}

pub(super) fn admits_omission(view: &str) -> bool {
    view.lines().any(|l| {
        let t = l.trim();
        let low = t.to_lowercase();
        t.starts_with("...") || t.starts_with('…') || t.contains(" … ")
            || ["omitted", "truncated", "hidden", "not shown", "elided", " similar]", "more groups", "more distinct"].iter().any(|w| low.contains(w))
            || plus_n_more(&low)
    })
}
