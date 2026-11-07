//! Patch text -> `Item`s. A hunk ends when its header's line counts are used up (so blank context lines, which the
//! cleaner turns into empty strings, and `git log -p` commit headers right after a hunk are both read correctly);
//! `--word-diff` hunks, which have no marker column and no usable counts, end at the next `@@`/`diff`/`commit`.
use super::model::*;

pub fn is_file_start(l: &str) -> bool { l.starts_with("diff --git ") || l.starts_with("diff --cc ") || l.starts_with("diff --combined ") }

pub fn is_commit(l: &str) -> bool {
    l.strip_prefix("commit ").map_or(false, |r| r.len() >= 40 && r[..40].bytes().all(|b| b.is_ascii_hexdigit()))
}

/// Per-parent and result line counts of `@@ -a,b +c,d @@` / `@@@ -a,b -c,d +e,f @@@`; None if it is not a header.
fn hunk_counts(l: &str) -> Option<(usize, Vec<usize>)> {
    let w = l.bytes().take_while(|&b| b == b'@').count();
    if w < 2 || l.as_bytes().get(w) != Some(&b' ') { return None; }
    let spec = l[w..].split(&"@".repeat(w)).next()?;
    let n = |s: &str| s.split(',').nth(1).map_or(Some(1), |x| x.trim().parse().ok());
    let (mut olds, mut new) = (vec![], None);
    for f in spec.split_whitespace() {
        if let Some(o) = f.strip_prefix('-') { olds.push(n(o)?); } else if let Some(p) = f.strip_prefix('+') { new = Some(n(p)?); }
    }
    if olds.len() != w - 1 { return None; }
    Some((new?, olds))
}

fn path_from_git_line(rest: &str) -> (String, String) {
    // `a/x b/x`: same path twice is the common case and survives spaces in names
    let half = rest.len() / 2;
    if rest.len() % 2 == 1 && rest.starts_with("a/") && rest[half + 1..].starts_with("b/") && rest[2..half] == rest[half + 3..] {
        return (rest[2..half].to_string(), rest[half + 3..].to_string());
    }
    let (a, b) = rest.split_once(" b/").unwrap_or((rest, rest));
    (a.trim_start_matches("a/").to_string(), b.to_string())
}

fn new_file(l: &str) -> FileDiff {
    let (old, new, combined) = if let Some(r) = l.strip_prefix("diff --git ") { let (a, b) = path_from_git_line(r); (a, b, false) }
        else { let p = l.splitn(3, ' ').nth(2).unwrap_or("").to_string(); (p.clone(), p, true) };
    FileDiff { old, new, status: Status::Mod, sim: None, binary: false, mode: None, combined, hunks: vec![] }
}

fn meta(f: &mut FileDiff, l: &str) {
    if l.starts_with("new file mode") { f.status = Status::Add; }
    else if l.starts_with("deleted file mode") { f.status = Status::Del; }
    else if let Some(p) = l.strip_prefix("rename from ").or_else(|| l.strip_prefix("copy from ")) { f.old = p.to_string(); }
    else if let Some(p) = l.strip_prefix("rename to ") { f.new = p.to_string(); f.status = Status::Rename; }
    else if let Some(p) = l.strip_prefix("copy to ") { f.new = p.to_string(); f.status = Status::Copy; }
    else if let Some(s) = l.strip_prefix("similarity index ") { f.sim = s.trim_end_matches('%').parse().ok(); }
    else if let Some(m) = l.strip_prefix("old mode ") { f.mode = Some(m.to_string()); }
    else if let Some(m) = l.strip_prefix("new mode ") { f.mode = Some(format!("{}->{m}", f.mode.take().unwrap_or_default())); }
    else if l.starts_with("Binary files ") || l.starts_with("GIT binary patch") { f.binary = true; }
    else if let Some(p) = l.strip_prefix("--- a/") { if f.status != Status::Add { f.old = p.to_string(); } }
    else if let Some(p) = l.strip_prefix("+++ b/") { f.new = p.to_string(); }
}

pub fn parse(lines: &[String]) -> Vec<Item> {
    let mut items: Vec<Item> = vec![];
    let mut text: Vec<String> = vec![];
    let mut i = 0;
    while i < lines.len() {
        let l = &lines[i];
        if !is_file_start(l) { text.push(l.clone()); i += 1; continue; }
        if !text.is_empty() { items.push(Item::Text(std::mem::take(&mut text))); }
        let mut f = new_file(l);
        i += 1;
        while i < lines.len() && !is_file_start(&lines[i]) && !is_commit(&lines[i]) {
            let l = &lines[i];
            let Some((new, olds)) = hunk_counts(l) else {
                if f.hunks.is_empty() { meta(&mut f, l); i += 1; continue; }
                break;
            };
            let width = olds.len();
            let (mut new, mut olds) = (new, olds);
            let mut body: Vec<String> = vec![];
            let mut word = false;
            i += 1;
            while i < lines.len() {
                let b = &lines[i];
                if is_file_start(b) || is_commit(b) || hunk_counts(b).is_some() { break; }
                if !word && new == 0 && olds.iter().all(|&o| o == 0) {
                    if b.starts_with('\\') { body.push(b.clone()); i += 1; }
                    break;
                }
                let p = b.get(..width.min(b.len())).unwrap_or("");
                if !word && (b.starts_with('~') || !p.chars().all(|c| c == ' ' || c == '+' || c == '-' || c == '\\')) { word = true; }
                if !word && !b.starts_with('\\') {
                    let pb = p.as_bytes();
                    for (j, o) in olds.iter_mut().enumerate() { if pb.get(j).copied().unwrap_or(b' ') != b'+' { *o = o.saturating_sub(1); } }
                    if !p.contains('-') { new = new.saturating_sub(1); }
                }
                body.push(b.clone());
                i += 1;
            }
            // plain --word-diff whose lines all happen to start with a space: no -/+ line, but inline markers
            if !word && body.iter().all(|b| !b.starts_with('+') && !b.starts_with('-')) && body.iter().any(|b| { let (a, d) = word_marks(b); a || d }) { word = true; }
            if word { body.retain(|b| b != "~"); }
            f.hunks.push(Hunk { head: l.clone(), body, width: if word { 0 } else { width } });
        }
        items.push(Item::File(f));
    }
    if !text.is_empty() { items.push(Item::Text(text)); }
    items
}
