//! PDF nodes: one node per document, plus heading nodes (kind "page", with the page number) when headings are
//! detectable, linked to the code symbols named in the text.
use super::edges::{put, EdgeMap};
use super::{Graph, Node, EXTRACTED};
use std::collections::HashMap;
use std::path::Path;

const MAX_PAGES: usize = 300;
const MAX_HEADINGS: usize = 60;

/// Text per page; empty when the file cannot be parsed (encrypted, scanned, damaged).
pub(crate) fn page_texts(path: &Path) -> Vec<String> {
    let run = || -> Option<Vec<String>> {
        let doc = lopdf::Document::load(path).ok()?;
        if doc.is_encrypted() { return None; }
        Some(doc.get_pages().iter().take(MAX_PAGES).map(|(&n, &id)| lines_of(&doc, n, id)).collect())
    };
    std::panic::catch_unwind(run).ok().flatten().unwrap_or_default()
}

fn num(o: &lopdf::Object) -> f32 { match o { lopdf::Object::Integer(i) => *i as f32, lopdf::Object::Real(r) => *r, _ => 0.0 } }

/// One page as text with a line break at every text-positioning move, so headings stay on their own line.
fn lines_of(doc: &lopdf::Document, n: u32, id: lopdf::ObjectId) -> String {
    use lopdf::Object;
    let ops = doc.get_page_content(id).ok().and_then(|b| lopdf::content::Content::decode(&b).ok()).map(|c| c.operations).unwrap_or_default();
    let mut out = String::new();
    let text = |bytes: &[u8], out: &mut String| out.extend(bytes.iter().map(|&b| b as char));
    for op in &ops {
        match op.operator.as_str() {
            "Tj" => if let Some(Object::String(b, _)) = op.operands.first() { text(b, &mut out) },
            "'" | "\"" => { out.push('\n'); if let Some(Object::String(b, _)) = op.operands.last() { text(b, &mut out) } }
            "TJ" => if let Some(Object::Array(a)) = op.operands.first() {
                for x in a { match x { Object::String(b, _) => text(b, &mut out), o if num(o) < -200.0 => out.push(' '), _ => {} } }
            },
            "T*" | "Tm" | "TD" | "ET" => out.push('\n'),
            "Td" if op.operands.get(1).map_or(false, |y| num(y) != 0.0) => out.push('\n'),
            _ => {}
        }
    }
    if out.chars().any(|c| c.is_alphabetic()) { out } else { doc.extract_text(&[n]).unwrap_or_default() }
}

/// A short line without sentence punctuation that is numbered ("2.1 Setup"), a "Chapter 3" style label, or ALL CAPS.
pub(crate) fn is_heading(line: &str) -> bool {
    let t = line.trim();
    let n = t.chars().count();
    if !(3..=80).contains(&n) || t.split_whitespace().count() > 10 || t.ends_with(['.', ',', ';', ':']) { return false; }
    let letters = t.chars().filter(|c| c.is_alphabetic()).count();
    if letters < 3 { return false; }
    let numbered = t.split_once(' ').map_or(false, |(h, rest)| h.trim_end_matches('.').split('.').all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit())) && rest.trim_start().starts_with(|c: char| c.is_uppercase()));
    let labeled = ["Chapter ", "Section ", "Appendix ", "Part "].iter().any(|p| t.starts_with(p) && t.chars().nth(p.len()).map_or(false, |c| c.is_ascii_digit() || c.is_uppercase()));
    numbered || labeled || (!t.chars().any(|c| c.is_lowercase()) && letters >= 4)
}

pub(crate) fn add(g: &mut Graph, rel: &str, path: &Path, size: u64, by_base: &HashMap<String, Vec<usize>>, m: &mut EdgeMap) -> usize {
    let pages = if size <= 20_000_000 { page_texts(path) } else { vec![] };
    g.nodes.push(Node { file: rel.into(), name: rel.into(), a: 1, b: pages.len(), kind: "pdf" });
    let pdf = g.nodes.len() - 1;
    let (mut cur, mut headings) = (pdf, 0);
    for (p, text) in pages.iter().enumerate() {
        for line in text.lines() {
            if headings < MAX_HEADINGS && is_heading(line) {
                g.nodes.push(Node { file: rel.into(), name: format!("{rel}#{}", line.trim()), a: p + 1, b: p + 1, kind: "page" });
                cur = g.nodes.len() - 1;
                headings += 1;
                put(m, (pdf, cur), EXTRACTED);
            }
            for to in super::text::names_in(line, &g.nodes, by_base) {
                put(m, (pdf, to), EXTRACTED);
                if cur != pdf { put(m, (cur, to), EXTRACTED); }
            }
        }
    }
    pdf
}
