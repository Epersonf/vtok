//! Splicing edited chunks back into the original file.
use super::chunks::Chunk;
use super::deletion::{attached_start, deletion_span};
use super::patch::token_patch;
use super::render::chunk_source;
use crate::language::Language;
use crate::{Result, VtokError};
use std::collections::{BTreeMap, HashMap};

/// edits: {chunk_id: vtok}. Returns the new source; unchanged chunks are not touched.
pub fn apply_edits(lang: &dyn Language, source: &str, chunks: &[Chunk], edits: &BTreeMap<String, String>,
                   table: &BTreeMap<String, String>, force: bool) -> Result<String> {
    let by_id: HashMap<&str, &Chunk> = chunks.iter().map(|c| (c.id.as_str(), c)).collect();
    let inv: HashMap<String, String> = table.iter().map(|(o, a)| (a.clone(), o.clone())).collect();
    let mut lines: Vec<String> = source.split_inclusive('\n').map(String::from).collect();
    let mut todo: Vec<(&Chunk, String)> = vec![];
    for (cid, vt) in edits {
        let Some(c) = by_id.get(cid.as_str()) else {
            let mut known: Vec<&str> = by_id.keys().copied().collect();
            known.sort();
            return Err(VtokError::new(format!("unknown chunk {cid:?}; known: {known:?}")));
        };
        let orig = chunk_source(source, c);
        let mut new = lang.decode_chunk(vt, &inv, c.member, c.indent, Some(&orig))?;
        if !force && lang.equivalent(&orig, &new, c.member) { continue; }
        let attempts = [patch_items(lang, &orig, vt, table, &inv, c), token_patch(lang, &orig, &new, c.member)];
        if let Some(p) = attempts.into_iter().flatten().find(|p| lang.equivalent(p, &new, c.member)) { new = p; }
        todo.push((c, new));
    }
    todo.sort_by(|a, b| b.0.start.cmp(&a.0.start));
    for w in todo.windows(2) {
        if w[1].0.end >= w[0].0.start { return Err(VtokError::new(format!("edits overlap: {} and {}", w[1].0.id, w[0].0.id))); }
    }
    for (c, new) in todo {
        if new.trim().is_empty() {
            let floor = chunks.iter().filter(|o| o.end < c.start).map(|o| o.end).max().unwrap_or(0);
            let (s, e) = deletion_span(lang.name(), &lines, c.start - 1, c.end, floor);
            lines.drain(s..e);
            continue;
        }
        let mut n = new;
        if !n.ends_with('\n') { n.push('\n'); }
        lines.splice(c.start - 1..c.end, [n]);
    }
    let out: String = lines.concat();
    lang.validate(&out)?;
    Ok(out)
}

/// Keep the original text (formatting, comments) of every top-level item whose Vtok line is unchanged:
/// only the changed run of items is replaced by decoded code. None = not applicable.
fn patch_items(lang: &dyn Language, orig: &str, vt: &str, table: &BTreeMap<String, String>,
               inv: &HashMap<String, String>, c: &Chunk) -> Option<String> {
    let spans = lang.item_spans(orig, c.member)?;
    let ov = lang.encode_chunk(orig, table, c.member).ok()?;
    let ol: Vec<&str> = ov.lines().filter(|l| !l.is_empty()).collect();
    let nl: Vec<&str> = vt.lines().filter(|l| !l.trim().is_empty()).collect();
    if ol.len() != spans.len() { return None; }
    let p = ol.iter().zip(&nl).take_while(|(a, b)| a == b).count();
    let s = ol[p..].iter().rev().zip(nl[p..].iter().rev()).take_while(|(a, b)| a == b).count();
    let (oe, ne) = (ol.len() - s, nl.len() - s);
    if p == ol.len() && p == nl.len() { return Some(orig.to_string()); }
    let pad = " ".repeat(c.indent);
    let new_mid = if ne > p {
        let d = lang.decode_chunk(&(nl[p..ne].join("\n") + "\n"), inv, c.member, c.indent, None).ok()?;
        d.strip_prefix(pad.as_str())?.strip_suffix('\n')?.to_string()
    } else { String::new() };
    let mut out = String::new();
    if oe > p {
        let (a, b) = (spans[p].0, spans[oe - 1].1);
        if new_mid.is_empty() {
            let a = attached_start(lang.name(), orig, if p > 0 { spans[p - 1].1 } else { 0 }, a);
            let b = if orig[b..].starts_with('\n') { b + 1 } else { b };
            let b = if orig[..a].ends_with("\n\n") && orig[b..].starts_with('\n') { b + 1 } else { b };
            out.push_str(&orig[..a]); out.push_str(&orig[b..]);
        } else {
            let region = &orig[a..b];
            let nm = token_patch(lang, region, &new_mid, c.member)
                .filter(|r| lang.equivalent(r, &new_mid, c.member)).unwrap_or(new_mid);
            out.push_str(&orig[..a]); out.push_str(&nm); out.push_str(&orig[b..]);
        }
    } else if p < spans.len() {
        let a = spans[p].0;
        out.push_str(&orig[..a]); out.push_str(&new_mid); out.push('\n'); out.push_str(&pad); out.push_str(&orig[a..]);
    } else {
        let b = spans.last()?.1;
        out.push_str(&orig[..b]); out.push('\n'); out.push_str(&pad); out.push_str(&new_mid); out.push_str(&orig[b..]);
    }
    Some(out)
}
