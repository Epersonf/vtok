//! Items -> entries (one per file, with its header line and tier) -> text. Commit headers (`git show`, `git log -p`)
//! are shortened to hash, author, date and message.
use super::model::*;
use super::plan::{take, Entry};
use super::rows::*;
use std::collections::HashMap;

const PREVIEW: usize = 10;
const MSG_LINES: usize = 12;

pub fn header(f: &FileDiff) -> String {
    let (a, d) = f.counts();
    let mut h = match f.status {
        Status::Add => format!("A {} (+{a})", f.new),
        Status::Del => format!("D {} (-{d})", f.old),
        Status::Rename | Status::Copy => {
            let sim = f.sim.map(|s| format!(" ({s}%)")).unwrap_or_default();
            let ch = if a + d > 0 { format!(" (+{a} -{d})") } else { String::new() };
            format!("{} {} -> {}{sim}{ch}", if f.status == Status::Rename { "R" } else { "C" }, f.old, f.new)
        }
        Status::Mod if a + d == 0 => format!("M {}", f.new),
        Status::Mod => format!("M {} (+{a} -{d})", f.new),
    };
    if f.combined { h.push_str(" merge"); }
    if let Some(m) = &f.mode { h.push_str(&format!(" mode {m}")); }
    if f.binary { h.push_str(" binary"); }
    else if a + d > 0 && f.generated() { h.push_str(" generated/lock, not shown"); }
    else if a + d > 0 && f.whitespace_only() { h.push_str(" whitespace/EOL-only, not shown"); }
    h
}

pub fn entry(f: &FileDiff, ctx: usize, seen: &mut HashMap<String, String>) -> Entry {
    let h = header(f);
    if f.binary || h.ends_with(", not shown") { return Entry::new(h, vec![], 0, 0, None); }
    match f.status {
        Status::Add => Entry::new(h, content_rows(f), 2, PREVIEW, symbols_line(f)),
        Status::Del => Entry::new(h, content_rows(f), 3, 0, None),
        _ => Entry::new(h, hunk_rows(f, ctx, seen), 1, 0, None),
    }
}

fn commit_text(t: &[String]) -> Vec<String> {
    let mut out = vec![];
    let mut msg = 0;
    let mut skipped = 0;
    for l in t {
        let s = l.trim();
        if s.is_empty() { continue; }
        if let Some(h) = l.strip_prefix("commit ") { out.push(format!("commit {}", h.chars().take(10).collect::<String>().trim())); msg = 0; }
        else if let Some(a) = l.strip_prefix("Author:") { out.push(format!("Author: {}", a.split('<').next().unwrap_or("").trim())); }
        else if l.starts_with("Date:") || l.starts_with("Merge:") { out.push(l.split_whitespace().collect::<Vec<_>>().join(" ")); }
        else if l.starts_with("    ") { msg += 1; if msg <= MSG_LINES { out.push(format!("  {}", cut(s))); } else { skipped += 1; } }
        else if s.contains(" | ") || (s.contains(" changed") && (s.contains("insertion") || s.contains("deletion"))) { continue; }
        else if out.len() < 40 { out.push(cut(s)); }
    }
    if skipped > 0 { out.push(format!("  … {skipped} more message lines")); }
    out
}

pub fn text_cost(items: &[Item]) -> usize {
    items.iter().map(|i| if let Item::Text(t) = i { commit_text(t).iter().map(|l| vtok_api::ntok(l) + 1).sum() } else { 0 }).sum()
}

fn note(e: &Entry, n: usize, shown: usize) -> String {
    let rest = e.total() - shown;
    let hunks = e.rows[n..].iter().filter(|r| r.head).count();
    let (a, d) = e.rows[n..].iter().filter(|r| r.changed == 1).fold((0, 0), |(a, d), r| if r.text.starts_with('+') { (a + 1, d) } else { (a, d + 1) });
    let hk = if hunks > 0 { format!(" in {hunks} more hunks") } else { String::new() };
    let pm = if a + d == rest { format!(" (+{a} -{d})") } else { String::new() };
    format!("  … {rest} more changed lines{pm}{hk}")
}

/// Render; returns the text and whether anything was left out.
pub fn render(items: &[Item], entries: &[Entry]) -> (String, bool) {
    let mut out = String::new();
    let mut cut_any = false;
    let mut k = 0;
    for it in items {
        match it {
            Item::Text(t) => for l in commit_text(t) { out.push_str(&l); out.push('\n'); },
            Item::File(_) => {
                let e = &entries[k];
                k += 1;
                out.push_str(&e.header);
                out.push('\n');
                let (n, shown) = take(&e.rows, e.cap.max(e.floor));
                for r in &e.rows[..n] { out.push_str(&r.text); out.push('\n'); }
                if shown < e.total() {
                    cut_any = true;
                    if e.tier == 3 && shown == 0 { continue; }
                    if let Some(x) = &e.extra { out.push_str(x); out.push('\n'); }
                    out.push_str(&note(e, n, shown));
                    out.push('\n');
                }
            }
        }
    }
    (out, cut_any)
}
