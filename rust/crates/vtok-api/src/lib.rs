//! High-level, string-based API shared by the CLI and the pip / npm bindings.
use std::collections::{BTreeMap, HashMap};
pub use vtok_core::lang::{EncodeOpts, Encoded, View};
pub use vtok_core::language::Language;
pub use vtok_core::split::{self, Chunk};
pub use vtok_core::{parse_table, render_table, tokens::ntok, Result, VtokError};

static PY: vtok_py::Python = vtok_py::Python;
static TS: vtok_ts::TypeScript = vtok_ts::TypeScript;
static RS: vtok_rs::Rust = vtok_rs::Rust;

pub fn languages() -> Vec<&'static dyn Language> { vec![&PY, &TS, &RS] }

pub fn language(name: &str) -> Result<&'static dyn Language> {
    languages().into_iter().find(|l| l.name() == name || (name == "ts" && l.name() == "typescript") || (name == "py" && l.name() == "python") || (name == "rs" && l.name() == "rust"))
        .ok_or_else(|| VtokError::new(format!("unknown language {name:?}; installed: {}", languages().iter().map(|l| l.name()).collect::<Vec<_>>().join(", "))))
}

pub fn language_for_path(path: &str) -> Result<&'static dyn Language> {
    languages().into_iter().find(|l| l.extensions().iter().any(|e| path.ends_with(e)))
        .ok_or_else(|| VtokError::new(format!("no language plugin for {path:?}")))
}

pub fn normalize_newlines(s: &str) -> String { s.replace("\r\n", "\n") }

#[derive(Default, Clone)]
pub struct Options {
    pub alias: Option<bool>,
    pub view: Option<String>,
    pub hide_literals: Option<usize>,
    pub table: Option<BTreeMap<String, String>>,
    pub verify: Option<bool>,
}

impl Options {
    pub fn to_opts(&self) -> Result<EncodeOpts> {
        let mut o = EncodeOpts::default();
        if let Some(a) = self.alias { o.alias = a; }
        if let Some(v) = &self.view { o.view = View::parse(v).ok_or_else(|| VtokError::new(format!("unknown view {v:?} (full|skeleton|lean)")))?; }
        if let Some(h) = self.hide_literals { o.hide_literals = h; }
        if let Some(v) = self.verify { o.verify = v; }
        o.table = self.table.clone();
        Ok(o)
    }
}

pub fn encode(lang: &str, source: &str, o: &Options) -> Result<Encoded> {
    language(lang)?.encode(source, &o.to_opts()?)
}

pub fn decode(lang: &str, vtok: &str, table: &HashMap<String, String>, literals: &HashMap<String, String>) -> Result<String> {
    language(lang)?.decode(vtok, table, literals)
}

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

/// Every identifier occurrence in the source as (name, 1-based line), from the grammar's leaf tokens
/// (comments and string contents are not identifiers). Used for name-based reference search.
pub fn idents(lang: &str, source: &str) -> Result<Vec<(String, usize)>> {
    Ok(idents_dot(lang, source)?.into_iter().map(|(n, l, _, _)| (n, l)).collect())
}

/// Like `idents`, plus whether the identifier directly follows a `.` (method or field access) and whether it is directly
/// followed by `(` (call form).
pub fn idents_dot(lang: &str, source: &str) -> Result<Vec<(String, usize, bool, bool)>> {
    let l = language(lang)?;
    let src = normalize_newlines(source);
    let spans = l.identifier_spans(&src).ok_or_else(|| VtokError::new("cannot tokenize this source (syntax error?)"))?;
    let mut line_starts = vec![0usize];
    for (i, b) in src.bytes().enumerate() { if b == b'\n' { line_starts.push(i + 1); } }
    let mut out = vec![];
    for (a, b) in spans {
        let Some(t) = src.get(a..b) else { continue };
        let mut cs = t.chars();
        if !cs.next().map_or(false, |c| c.is_alphabetic() || c == '_' || c == '$') || !t.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '$') { continue; }
        let line = line_starts.partition_point(|&s| s <= a);
        let dot = src.get(..a).map_or(false, |h| h.trim_end().ends_with('.'));
        let call = src.get(b..).map_or(false, |t| t.trim_start_matches(|c: char| c == ' ' || c == '\t').starts_with('(')) || src.get(b..).map_or(false, |t| t.starts_with("::<"));
        out.push((t.to_string(), line, dot, call));
    }
    Ok(out)
}
