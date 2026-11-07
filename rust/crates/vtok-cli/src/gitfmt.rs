//! Dedicated views for `git diff` / `git show` and `git log` output. The generic summariser is built for logs and test
//! runs; on a diff it would keep arbitrary lines, so these two get their own: a diff keeps every file (name, +/- counts)
//! and the changed lines of each hunk without unchanged context; a log keeps one line per commit.
use vtok_api::ntok;

fn is_diff(lines: &[String]) -> bool { lines.iter().take(40).any(|l| l.starts_with("diff --git ")) }
fn is_log(lines: &[String]) -> bool {
    lines.first().map_or(false, |l| l.starts_with("commit ") && l.len() >= 47 && l[7..47].chars().all(|c| c.is_ascii_hexdigit()))
}

struct FileDiff { name: String, tag: String, add: usize, del: usize, hunks: Vec<(String, Vec<String>)> }

fn parse_diff(lines: &[String]) -> (Vec<String>, Vec<FileDiff>) {
    let mut head: Vec<String> = vec![];
    let mut files: Vec<FileDiff> = vec![];
    for l in lines {
        if let Some(rest) = l.strip_prefix("diff --git ") {
            let name = rest.split(" b/").last().unwrap_or(rest).trim().to_string();
            files.push(FileDiff { name, tag: String::new(), add: 0, del: 0, hunks: vec![] });
            continue;
        }
        let Some(f) = files.last_mut() else { head.push(l.clone()); continue };
        if l.starts_with("@@") { f.hunks.push((l.clone(), vec![])); }
        else if l.starts_with("new file mode") { f.tag = "new".into(); }
        else if l.starts_with("deleted file mode") { f.tag = "deleted".into(); }
        else if l.starts_with("rename from ") { f.tag = format!("renamed from {}", &l[12..]); }
        else if l.starts_with("Binary files") || l.starts_with("GIT binary patch") { f.tag = "binary".into(); }
        else if let Some(h) = f.hunks.last_mut() {
            if l.starts_with('+') && !l.starts_with("+++") { f.add += 1; h.1.push(l.clone()); }
            else if l.starts_with('-') && !l.starts_with("---") { f.del += 1; h.1.push(l.clone()); }
        }
    }
    (head, files)
}

fn render_diff(head: &[String], files: &[FileDiff], cap: usize, shown_files: usize) -> String {
    let mut out = String::new();
    for l in head { out.push_str(l); out.push('\n'); }
    for (i, f) in files.iter().enumerate() {
        let tag = if f.tag.is_empty() { String::new() } else { format!(" [{}]", f.tag) };
        out.push_str(&format!("{} (+{} -{}){}\n", f.name, f.add, f.del, tag));
        if i >= shown_files { continue; }
        let mut left = cap;
        let total: usize = f.hunks.iter().map(|h| h.1.len()).sum();
        for (hh, body) in &f.hunks {
            if left == 0 { break; }
            // keep only the part of the hunk header after the second @@ (the enclosing function), plus the line number
            let at = hh.split("@@").nth(1).unwrap_or("").trim().to_string();
            let ctx = hh.split("@@").nth(2).unwrap_or("").trim();
            let line_no = at.split_whitespace().nth(1).unwrap_or("").trim_start_matches('+').split(',').next().unwrap_or("").to_string();
            out.push_str(&format!("  @{}{}{}\n", line_no, if ctx.is_empty() { "" } else { " " }, ctx));
            for l in body.iter().take(left) { out.push_str("  "); out.push_str(&l.chars().take(200).collect::<String>()); out.push('\n'); }
            left = left.saturating_sub(body.len());
        }
        if total > cap { out.push_str(&format!("  … {} more changed lines\n", total - cap)); }
    }
    out
}

pub fn diff_view(lines: &[String], budget: usize) -> Option<String> {
    if !is_diff(lines) { return None; }
    let (mut head, files) = parse_diff(lines);
    head.truncate(14);
    let head: Vec<String> = head.into_iter().filter(|l| !l.trim().is_empty()).collect();
    let mut cap = 80;
    loop {
        let r = render_diff(&head, &files, cap, files.len());
        if ntok(&r) <= budget { return Some(r); }
        if cap > 6 { cap /= 2; continue; }
        // still too big: show the changed lines for as many files as fit, name and counts for the rest
        let mut n = files.len();
        while n > 0 { n -= 1; let r = render_diff(&head, &files, 6, n); if ntok(&r) <= budget { return Some(r); } }
        return Some(render_diff(&head, &files, 0, 0));
    }
}

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
    fn diff_keeps_files_counts_and_changed_lines_only() {
        let d = l("diff --git a/src/a.rs b/src/a.rs\nindex 1..2 100644\n--- a/src/a.rs\n+++ b/src/a.rs\n@@ -10,4 +10,4 @@ fn alpha() {\n context one\n-old line\n+new line\n context two\ndiff --git a/b.txt b/b.txt\nnew file mode 100644\n--- /dev/null\n+++ b/b.txt\n@@ -0,0 +1,2 @@\n+x\n+y");
        let v = diff_view(&d, 500).unwrap();
        assert!(v.contains("src/a.rs (+1 -1)") && v.contains("b.txt (+2 -0) [new]"), "{v}");
        assert!(v.contains("@10 fn alpha()") && v.contains("-old line") && v.contains("+new line") && !v.contains("context one"), "{v}");
    }

    #[test]
    fn big_diff_degrades_to_names_and_counts() {
        let mut t = String::new();
        for i in 0..30 { t += &format!("diff --git a/f{i}.rs b/f{i}.rs\n--- a/f{i}.rs\n+++ b/f{i}.rs\n@@ -1,1 +1,40 @@\n"); for j in 0..40 { t += &format!("+line {i} {j} with some words in it\n"); } }
        let v = diff_view(&l(&t), 300).unwrap();
        assert!(ntok(&v) <= 300 && v.contains("f0.rs (+40 -0)") && v.contains("f29.rs (+40 -0)"), "{v}");
    }

    #[test]
    fn log_is_one_line_per_commit() {
        let g = l("commit 0123456789abcdef0123456789abcdef01234567\nAuthor: Ann <a@x.y>\nDate:   Fri Nov 6 12:45:00 2026 +0000\n\n    Fix the thing\n\n    Longer body that goes away.\n\n 3 files changed, 10 insertions(+), 2 deletions(-)\n\ncommit 89abcdef0123456789abcdef0123456789abcdef\nAuthor: Bo <b@x.y>\nDate:   Thu Nov 5 09:00:00 2026 +0000\n\n    Second");
        let v = log_view(&g, 200).unwrap();
        assert!(v.contains("0123456 Nov 6 2026 Ann: Fix the thing [3 files changed, 10 insertions(+), 2 deletions(-)]") && v.contains("89abcde Nov 5 2026 Bo: Second") && !v.contains("Longer body"), "{v}");
    }
}
