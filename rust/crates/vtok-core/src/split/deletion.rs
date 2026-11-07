//! What deleting an item takes with it: the doc comments and attributes touching it from above (left behind, they would
//! describe or change the next item instead) and one of the two blank lines that would otherwise sit between its neighbours.
//! Plain comments stay: a licence or module header sits right above the first item more often than a note about it.

/// A line that belongs to the item below it: a doc comment (`///`, `/** ... */`, or `//` in Go, where docs are plain
/// comments) or an attribute, annotation or decorator (`#[..]`, `@Name`, `[Name]`).
fn is_attached_line(line: &str, lang: &str) -> bool {
    let t = line.trim();
    t.starts_with("///") || t.starts_with("/**") || t.starts_with('*') || t.starts_with("#[") || t.starts_with('@')
        || (t.starts_with('[') && t.ends_with(']')) || (lang == "go" && t.starts_with("//"))
}

/// The lines to remove for the item on `lines[start..end]`: the item, the attached lines right above it (not above index
/// `floor`, where the previous chunk ends), and a blank line when both sides of the hole already have one.
pub(super) fn deletion_span(lang: &str, lines: &[String], start: usize, end: usize, floor: usize) -> (usize, usize) {
    let blank = |i: usize| lines.get(i).map_or(false, |l| l.trim().is_empty());
    let mut s = start;
    while s > floor && is_attached_line(&lines[s - 1], lang) { s -= 1; }
    let mut e = end;
    if e >= lines.len() {
        while s > floor && blank(s - 1) { s -= 1; }
    } else if s > 0 && blank(s - 1) && blank(e) {
        e += 1;
    }
    (s, e)
}

/// Byte offset where the lines of the item that starts at `at` in `text` begin: its first line and the attached lines right above
/// it, but never back into the previous item, which ended at `prev_end`.
pub(super) fn attached_start(lang: &str, text: &str, prev_end: usize, at: usize) -> usize {
    let mut start = text[..at].rfind('\n').map_or(0, |i| i + 1);
    while start > prev_end {
        let above_end = start - 1;
        let above_start = text[..above_end].rfind('\n').map_or(0, |i| i + 1);
        if above_start < prev_end || !is_attached_line(&text[above_start..above_end], lang) { break; }
        start = above_start;
    }
    start
}
