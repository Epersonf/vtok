//! Grouped diagnostics for compilers and linters (tsc, eslint, ruff, mypy, ...): the message once, then where it
//! happens. Identical (code, message, elaboration) diagnostics form one group with a compact list of lines; groups of one
//! that differ only in a quoted name share a template. Nothing distinct is dropped: past the group cap the remaining
//! kinds are still named with their counts, and an overlong place list says how many places it left out.
mod cap;
mod group;
mod locs;
mod path;
mod quote;
#[cfg(test)] mod t_diag;

use group::{group, Group};
pub use cap::cap_classes;
pub use path::set_base;

/// One diagnostic. `extra` are the indented elaboration lines tsc prints under a message.
#[derive(Clone, Default)]
pub struct D { pub file: String, pub line: u32, pub col: u32, pub code: String, pub msg: String, pub extra: Vec<String>, pub warn: bool }

impl D {
    pub fn new(file: &str, line: u32, col: u32, code: &str, msg: &str) -> D {
        D { file: path::relative(file), line, col, code: code.into(), msg: clip(msg, MAX_MSG), ..Default::default() }
    }
}

/// A message longer than this (tsc prints whole type literals) is cut: the start says what it is about.
const MAX_MSG: usize = 300;

fn clip(s: &str, n: usize) -> String {
    if s.chars().count() <= n { s.to_string() } else { format!("{}...", s.chars().take(n).collect::<String>()) }
}

const MAX_GROUPS: usize = 40;
const MAX_PLACES: usize = 150;
const MAX_WARN_PLACES: usize = 12;
/// Places listed over all groups; once they are used up every group still lists `MIN_PLACES` and counts the rest.
const TOTAL_PLACES: usize = 400;
const MIN_PLACES: usize = 5;
const MAX_EXTRA: usize = 6;

pub fn plural(n: usize, one: &str, many: &str) -> String { format!("{n} {}", if n == 1 { one } else { many }) }

/// Words joined by single spaces, empty ones (no file, no rule code) left out.
fn words(parts: &[&str]) -> String { parts.iter().filter(|p| !p.is_empty()).copied().collect::<Vec<_>>().join(" ") }

/// One group's lines, at most about `cap` places; returns how many places it listed.
fn push_group(out: &mut Vec<String>, g: &Group, cap: usize) -> usize {
    let w = if g.warn { "warning" } else { "" };
    let mut extra: Vec<String> = g.extra.iter().take(MAX_EXTRA).map(|e| format!("  {}", clip(e, MAX_MSG))).collect();
    if g.extra.len() > MAX_EXTRA { extra.push("  ...".into()); }
    if g.hits.len() == 1 && g.hits[0].arg.is_empty() {
        let h = &g.hits[0];
        let at = if h.line == 0 { h.file.clone() } else { format!("{}:{}", h.file, h.line) };
        out.push(words(&[&at, w, &g.code, &g.msg]));
        out.extend(extra);
        return 1;
    }
    let count = format!("x{}:", g.hits.len());
    let head = if g.code.is_empty() { words(&[w, &count, &g.msg]) } else { words(&[w, &format!("{} {count}", g.code), &g.msg]) };
    out.push(head);
    out.extend(extra);
    out.extend(locs::lines(g, cap));
    locs::shown(g.hits.len(), cap)
}

/// Kinds past the cap, still named with their first place: `+2 more kinds (13 diagnostics): TS2304 x12 a.ts:4, TS2339 b.ts:9`.
fn tally(rest: &[Group], noun: (&str, &str)) -> String {
    let mut s = String::new();
    for g in rest {
        let h = &g.hits[0];
        let at = if h.line == 0 { h.file.clone() } else { format!("{}:{}", h.file, h.line) };
        let label = if g.code.is_empty() { clip(&g.msg, 40) } else { g.code.clone() };
        let one = words(&[&label, &if g.hits.len() > 1 { format!("x{}", g.hits.len()) } else { String::new() }, &at]);
        if s.len() + one.len() > 900 { s.push_str(", ..."); break; }
        if !s.is_empty() { s.push_str(", "); }
        s.push_str(&one);
    }
    format!("+{} more kinds ({}): {s}", rest.len(), plural(rest.iter().map(|g| g.hits.len()).sum(), noun.0, noun.1))
}

/// Lines for `ds`: groups (errors first), then the kinds past the cap, then `summary` or `Found N <noun> in M files.`
pub fn render(ds: &[D], noun: (&str, &str), summary: Option<String>) -> Vec<String> {
    let mut groups = group(ds);
    groups.sort_by_key(|g| g.warn); // stable: errors keep their order, warnings follow
    let mut out = vec![];
    let mut room = TOTAL_PLACES;
    for g in groups.iter().take(MAX_GROUPS) {
        let cap = (if g.warn { MAX_WARN_PLACES } else { MAX_PLACES }).min(room.max(MIN_PLACES));
        room = room.saturating_sub(push_group(&mut out, g, cap));
    }
    if groups.len() > MAX_GROUPS { out.push(tally(&groups[MAX_GROUPS..], noun)); }
    let files: std::collections::HashSet<&str> = groups.iter().flat_map(|g| g.hits.iter().map(|h| h.file.as_str())).filter(|f| !f.is_empty()).collect();
    let total: usize = groups.iter().map(|g| g.hits.len()).sum();
    let in_files = if files.is_empty() { String::new() } else { format!(" in {}", plural(files.len(), "file", "files")) };
    out.push(summary.unwrap_or_else(|| format!("Found {}{in_files}.", plural(total, noun.0, noun.1))));
    out
}
