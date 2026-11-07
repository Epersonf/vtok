//! pytest report -> one `Item` per failing or erroring test: where it failed, which test, the decisive message.
use super::py_pytest::{Block, Entry, Report};
use super::util::short;

pub(super) struct Item { pub lead: String, pub file: String, pub line: Option<String>, pub name: String, pub msg: String }

/// `TestFoo.test_bar` -> `TestFoo::test_bar` (dots inside `[...]` stay).
fn node_name(title: &str) -> String {
    let (mut depth, mut out) = (0, String::new());
    for c in title.chars() {
        match c { '[' => depth += 1, ']' => depth -= 1, _ => {} }
        if c == '.' && depth == 0 { out.push_str("::"); } else { out.push(c); }
    }
    out
}

fn tail(nodeid: &str) -> &str { nodeid.split_once("::").map_or(nodeid, |x| x.1) }

fn rel(path: &str, root: &str) -> String {
    let p = path.replace('\\', "/");
    match p.strip_prefix(root) { Some(r) if !root.is_empty() => r.trim_start_matches('/').to_string(), _ => p }
}

/// The decisive `E` lines: the first three, minus pytest's own hints and the `-v` full diff; a syntax-error report
/// ends in its last line.
pub(super) fn message(e: &[String]) -> String {
    let end = e.iter().position(|l| l.starts_with("Full diff:")).unwrap_or(e.len());
    let useful: Vec<&String> = e[..end].iter().filter(|l| !l.is_empty() && !l.contains("Use -v") && !l.starts_with("Omitting ")).collect();
    let picked: Vec<&str> = if useful.first().map_or(false, |l| l.starts_with("File \"")) { useful.last().into_iter().map(|l| l.as_str()).collect() } else { useful.iter().take(3).map(|l| l.as_str()).collect() };
    let more = if useful.len() > 3 && picked.len() == 3 { " ..." } else { "" };
    short(&format!("{}{more}", picked.join(" | ")), 220)
}

/// Does this FAILURES / ERRORS block belong to that short-summary entry?
fn titled(b: &Block, e: &Entry) -> bool {
    let t = &b.title;
    if let Some(p) = t.strip_prefix("ERROR collecting ") { return e.status == "ERROR" && e.nodeid == p.replace('\\', "/"); }
    let (status, name) = match (t.strip_prefix("ERROR at setup of "), t.strip_prefix("ERROR at teardown of ")) { (Some(n), _) | (_, Some(n)) => ("ERROR", n), _ => ("FAILED", t.as_str()) };
    e.status == status && (tail(&e.nodeid) == node_name(name) || tail(&e.nodeid) == name)
}

/// `--tb=line` blocks have no title: match on the message, which the summary may have cut with `...`.
fn by_message(b: &Block, e: &Entry) -> bool {
    let (a, m) = (message(&b.e), e.msg.trim_end_matches("...").trim_end());
    !m.is_empty() && (a.starts_with(m) || m.starts_with(a.as_str()))
}

fn item(b: Option<&Block>, e: Option<&Entry>, root: &str) -> Item {
    let title = b.map_or("", |b| b.title.as_str());
    let lead = if title.starts_with("ERROR collecting") { "ERROR collect " } else if title.starts_with("ERROR at setup") { "ERROR setup " } else if title.starts_with("ERROR at teardown") { "ERROR teardown " } else if e.map_or(false, |e| e.status == "ERROR") { "ERROR " } else { "" };
    let loc = b.and_then(|b| b.location());
    let nodeid_file = e.map(|e| e.nodeid.split("::").next().unwrap_or("").to_string());
    let file = match (&nodeid_file, &loc) {
        (Some(f), Some((p, _))) if rel(p, root).ends_with(f.as_str()) => f.clone(),
        (_, Some((p, _))) => rel(p, root),
        (Some(f), None) => f.clone(),
        _ => String::new(),
    };
    let own_name = title.trim_start_matches("ERROR at setup of ").trim_start_matches("ERROR at teardown of ");
    let name = if lead == "ERROR collect " { String::new() } else { e.filter(|e| e.nodeid.contains("::")).map(|e| tail(&e.nodeid).to_string()).unwrap_or_else(|| own_name.to_string()) };
    let e_msg = b.map(|b| message(&b.e)).unwrap_or_default();
    let msg = if e_msg.is_empty() { e.map(|e| e.msg.clone()).unwrap_or_default() } else { e_msg };
    Item { lead: lead.into(), file, line: loc.map(|l| l.1), name, msg }
}

fn entry_only(e: &Entry) -> Item {
    let (file, name) = e.nodeid.split_once("::").map_or((e.nodeid.clone(), String::new()), |(f, n)| (f.to_string(), n.to_string()));
    Item { lead: if e.status == "ERROR" { "ERROR ".into() } else { String::new() }, file, line: None, name, msg: e.msg.clone() }
}

pub(super) fn items(rep: &Report) -> Vec<Item> {
    let flat = !rep.blocks.is_empty() && rep.blocks.iter().all(|b| b.title.is_empty());
    let mut used = vec![false; rep.blocks.len()];
    let mut out = vec![];
    if flat {
        // order of the summary; each entry takes the location line whose message it starts
        for e in &rep.entries {
            let hit = (0..rep.blocks.len()).find(|&i| !used[i] && by_message(&rep.blocks[i], e));
            if let Some(i) = hit { used[i] = true; }
            out.push(item(hit.map(|i| &rep.blocks[i]), Some(e), &rep.rootdir));
        }
        out.extend(rep.blocks.iter().zip(&used).filter(|(_, u)| !**u).map(|(b, _)| item(Some(b), None, &rep.rootdir)));
        return out;
    }
    let mut taken = vec![false; rep.entries.len()];
    for b in &rep.blocks {
        let hit = rep.entries.iter().enumerate().position(|(k, e)| !taken[k] && titled(b, e));
        if let Some(k) = hit { taken[k] = true; }
        out.push(item(Some(b), hit.map(|k| &rep.entries[k]), &rep.rootdir));
    }
    // summary lines with no block: --tb=no, or the section was cut
    out.extend(rep.entries.iter().zip(&taken).filter(|(_, t)| !**t).map(|(e, _)| entry_only(e)));
    out
}
