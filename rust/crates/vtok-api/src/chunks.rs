//! Big files as chunks: the index, symbols and outline, one chunk or symbol, applying edited chunks, a project alias table.
use crate::{language, normalize_newlines, split, Chunk, EncodeOpts, Result, VtokError};
use std::collections::BTreeMap;

pub struct SplitResult {
    pub index: String,
    pub chunks: Vec<Chunk>,
    pub table: BTreeMap<String, String>,
    pub whole_file: bool,
}

/// Index (skeleton + chunk map) for big files; the full view for files at most `max_lines` long.
pub fn split_file(lang: &str, source: &str, max_lines: usize) -> Result<SplitResult> {
    let l = language(lang)?;
    let src = normalize_newlines(source);
    let chunks = split::plan_chunks(l, &src, max_lines)?;
    let table = split::file_table(l, &src);
    if chunks.len() == 1 {
        let vt = l.encode(&src, &EncodeOpts { table: Some(table.clone()), ..Default::default() })?.vtok;
        return Ok(SplitResult { index: vt, chunks, table, whole_file: true });
    }
    let index = split::render_index(l, &src, &chunks, &table)?;
    Ok(SplitResult { index, chunks, table, whole_file: false })
}

/// Addressable symbols of a file: (id, first line, last line), 1-based inclusive.
pub fn symbols(lang: &str, source: &str) -> Result<Vec<(String, usize, usize)>> {
    let l = language(lang)?;
    let src = normalize_newlines(source);
    Ok(split::plan_symbols(l, &src)?.into_iter().map(|c| (c.id, c.start, c.end)).collect())
}

/// Names-only outline: one `name Lstart-end` line per symbol (imports merged into one line). The cheapest way for a
/// model to see what a file contains before asking for one symbol with `chunk_of`.
pub fn outline(lang: &str, source: &str) -> Result<String> {
    let mut out = String::new();
    let mut imp: Option<(usize, usize)> = None;
    let span = |a: usize, b: usize| if a == b { format!("L{a}") } else { format!("L{a}-{b}") };
    for (id, a, b) in symbols(lang, source)? {
        let is_import = id == "use" || id.starts_with("use#") || id.starts_with("import");
        if is_import {
            imp = Some(imp.map_or((a, b), |(x, _)| (x, b)));
            continue;
        }
        if let Some((x, y)) = imp.take() { out.push_str(&format!("imports {}\n", span(x, y))); }
        out.push_str(&format!("{id} {}\n", span(a, b)));
    }
    if let Some((x, y)) = imp { out.push_str(&format!("imports {}\n", span(x, y))); }
    Ok(out)
}

pub fn chunk_of(lang: &str, source: &str, id: &str, max_lines: usize) -> Result<String> {
    let l = language(lang)?;
    let src = normalize_newlines(source);
    let chunks = split::resolve_chunks(l, &src, max_lines, &[id])?;
    let table = split::file_table(l, &src);
    let Some(c) = chunks.iter().find(|c| c.id == id) else {
        return Err(VtokError::new(format!("unknown chunk {id:?}; known: {}", chunks.iter().map(|c| c.id.as_str()).collect::<Vec<_>>().join(", "))));
    };
    split::chunk_vtok(l, &src, c, &table)
}

/// Like `chunk_of`, plus the part of the file's alias table (original -> alias) that the chunk actually uses,
/// so a chunk can be decoded without having read the rest of the file.
pub fn chunk_with_aliases(lang: &str, source: &str, id: &str, max_lines: usize) -> Result<(String, BTreeMap<String, String>)> {
    let v = chunk_of(lang, source, id, max_lines)?;
    let l = language(lang)?;
    let table = split::file_table(l, &normalize_newlines(source));
    let words: std::collections::HashSet<&str> = v.split(|c: char| !(c.is_alphanumeric() || c == '_' || c == '$')).filter(|w| !w.is_empty()).collect();
    let used = table.into_iter().filter(|(_, a)| words.contains(a.as_str())).collect();
    Ok((v, used))
}

pub fn apply_chunks(lang: &str, source: &str, edits: &BTreeMap<String, String>, max_lines: usize, force: bool) -> Result<String> {
    let l = language(lang)?;
    let src = normalize_newlines(source);
    let ids: Vec<&str> = edits.keys().map(|k| k.as_str()).collect();
    let chunks = split::resolve_chunks(l, &src, max_lines, &ids)?;
    let table = split::file_table(l, &src);
    split::apply_edits(l, &src, &chunks, edits, &table, force)
}

/// One stable alias table for many files of the same language.
pub fn project_table(lang: &str, sources: &[&str]) -> Result<BTreeMap<String, String>> {
    Ok(split::plan_project(language(lang)?, sources, 2))
}
