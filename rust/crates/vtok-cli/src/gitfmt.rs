//! Dedicated view for `git log` output: one line per commit. (Patches: see gitdiff.rs.)
use vtok_api::ntok;

fn is_log(lines: &[String]) -> bool {
    lines.first().map_or(false, |l| l.starts_with("commit ") && l.len() >= 47 && l[7..47].chars().all(|c| c.is_ascii_hexdigit()))
}

/// Patches have their own module (`gitdiff`); re-exported here so the pipeline's call site stays the same.
pub use crate::gitdiff::diff_view;

pub fn log_view(lines: &[String], budget: usize) -> Option<String> {
    if !is_log(lines) { return None; }
    let mut out: Vec<String> = vec![];
    let (mut hash, mut author, mut date, mut subject, mut stat): (String, String, String, String, String) = Default::default();
    let flush = |h: &mut String, a: &mut String, d: &mut String, s: &mut String, st: &mut String, out: &mut Vec<String>| {
        if h.is_empty() { return; }
        let who = a.split('<').next().unwrap_or("").trim();
        out.push(format!("{} {} {}: {}{}", &h[..h.len().min(7)], d, who, s, if st.is_empty() { String::new() } else { format!(" [{st}]") }));
        h.clear(); a.clear(); d.clear(); s.clear(); st.clear();
    };
    for l in lines {
        if let Some(h) = l.strip_prefix("commit ") { flush(&mut hash, &mut author, &mut date, &mut subject, &mut stat, &mut out); hash = h.split_whitespace().next().unwrap_or("").to_string(); }
        else if let Some(a) = l.strip_prefix("Author:") { author = a.trim().to_string(); }
        else if let Some(d) = l.strip_prefix("Date:") { let w: Vec<&str> = d.split_whitespace().collect(); date = if w.len() >= 5 { format!("{} {} {}", w[1], w[2], w[4]) } else { d.trim().to_string() }; }
        else if l.starts_with("    ") && subject.is_empty() && !l.trim().is_empty() { subject = l.trim().to_string(); }
        else if l.contains(" changed, ") && (l.contains("insertion") || l.contains("deletion")) { stat = l.trim().to_string(); }
    }
    flush(&mut hash, &mut author, &mut date, &mut subject, &mut stat, &mut out);
    let total = out.len();
    let mut keep = String::new();
    let mut shown = 0;
    for l in &out { if ntok(&keep) + ntok(l) > budget { break; } keep.push_str(l); keep.push('\n'); shown += 1; }
    if shown < total { keep.push_str(&format!("… {} older commits not shown\n", total - shown)); }
    Some(keep)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn l(t: &str) -> Vec<String> { t.lines().map(String::from).collect() }

    #[test]
    fn log_is_one_line_per_commit() {
        let g = l("commit 0123456789abcdef0123456789abcdef01234567\nAuthor: Ann <a@x.y>\nDate:   Fri Nov 6 12:45:00 2026 +0000\n\n    Fix the thing\n\n    Longer body that goes away.\n\n 3 files changed, 10 insertions(+), 2 deletions(-)\n\ncommit 89abcdef0123456789abcdef0123456789abcdef\nAuthor: Bo <b@x.y>\nDate:   Thu Nov 5 09:00:00 2026 +0000\n\n    Second");
        let v = log_view(&g, 200).unwrap();
        assert!(v.contains("0123456 Nov 6 2026 Ann: Fix the thing [3 files changed, 10 insertions(+), 2 deletions(-)]") && v.contains("89abcde Nov 5 2026 Bo: Second") && !v.contains("Longer body"), "{v}");
    }
}
