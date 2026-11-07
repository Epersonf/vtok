//! pytest report -> lines: counts, skip/xfail notes, then one line per failing test (`file:line name: message`).
use super::py_pytest::Report;
use super::py_pytest_items::{items, Item};

const MAX_LINES: usize = 40;
const FOLD_FROM: usize = 6;

impl Item {
    fn text(&self) -> String {
        let at = match &self.line { Some(n) => format!("{}:{n}", self.file), None => self.file.clone() };
        let name = if self.name.is_empty() { String::new() } else { format!(" {}", self.name) };
        let sep = if self.msg.is_empty() { "" } else if self.name.is_empty() { " " } else { ": " };
        format!("{}{at}{name}{sep}{}", self.lead, self.msg)
    }

    /// Same failure site and message shape, digits aside: parametrized cases of one test.
    fn base(&self) -> Option<(String, Option<String>, String, String)> {
        let (b, _) = self.name.split_once('[')?;
        let mut shape = String::new();
        for c in self.msg.chars() { if !c.is_ascii_digit() { shape.push(c); } else if !shape.ends_with('#') { shape.push('#'); } }
        Some((self.file.clone(), self.line.clone(), b.to_string(), shape))
    }
}

/// Parametrized failures of one test with one message shape: the first in full, the rest by id.
fn lines_of(items: Vec<Item>) -> Vec<String> {
    let mut out = vec![];
    let mut done = vec![false; items.len()];
    for i in 0..items.len() {
        if done[i] { continue; }
        out.push(items[i].text());
        let Some(key) = items[i].base() else { continue };
        let same: Vec<usize> = (i + 1..items.len()).filter(|&j| !done[j] && items[j].base().as_ref() == Some(&key)).collect();
        if same.len() + 1 < FOLD_FROM { continue; }
        let ids: Vec<&str> = same.iter().map(|&j| items[j].name.split_once('[').map_or("", |x| x.1.trim_end_matches(']'))).collect();
        let list = if ids.len() > 6 { format!("{}, ... {}", ids[..5].join(", "), ids[ids.len() - 1]) } else { ids.join(", ") };
        out.push(format!("  +{} more {}[..] with the same message: {list}", same.len(), key.2));
        for j in same { done[j] = true; }
    }
    out
}

fn listed(label: &str, v: &[String], n: usize) -> String {
    let more = if v.len() > n { format!(", +{} more", v.len() - n) } else { String::new() };
    format!("{label}: {}{more}", v.iter().take(n).cloned().collect::<Vec<_>>().join(", "))
}

pub(super) fn render(rep: &Report) -> Vec<String> {
    let mut out = vec![format!("pytest: {}", rep.counts)];
    if !rep.skipped.is_empty() {
        let v: Vec<String> = rep.skipped.iter().map(|(r, n)| format!("{n} {}", if r.is_empty() { "(no reason)" } else { r })).collect();
        out.push(listed("skipped", &v, 8));
    }
    for (label, v) in [("xfailed", &rep.xfail), ("xpassed", &rep.xpass)] {
        if !v.is_empty() { out.push(listed(label, v, 6)); }
    }
    out.extend(rep.notes.iter().cloned());
    let lines = lines_of(items(rep));
    let total = lines.len();
    out.extend(lines.into_iter().take(MAX_LINES));
    if total > MAX_LINES { out.push(format!("... +{} more failures", total - MAX_LINES)); }
    let captured = rep.blocks.iter().filter(|b| b.captured).count();
    if captured > 0 { out.push(format!("(captured output of {captured} tests omitted)")); }
    out
}
