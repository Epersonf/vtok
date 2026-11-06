//! Node.js bindings. Tables are plain objects `{original: alias}` (the object `encode` returns).
use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::collections::{BTreeMap, HashMap};
use vtok_api as api;

fn err(e: api::VtokError) -> Error { Error::from_reason(e.to_string()) }

fn inv(t: &Option<HashMap<String, String>>) -> HashMap<String, String> {
    t.as_ref().map(|m| m.iter().map(|(o, a)| (a.clone(), o.clone())).collect()).unwrap_or_default()
}

#[napi(object)]
pub struct LanguageInfo {
    pub name: String,
    pub extensions: Vec<String>,
}

#[napi(object)]
pub struct EncodeOptions {
    /// Shorten frequent names with an alias table (default true).
    pub alias: Option<bool>,
    /// "full" (lossless), "skeleton" (signatures only) or "lean" (no types / import paths); read-only views.
    pub view: Option<String>,
    /// Hide string literals of at least this many tokens behind placeholders (0 = off).
    pub hide_literals: Option<u32>,
    /// Fixed alias table `{original: alias}` (e.g. a project table); disables per-file planning.
    pub table: Option<HashMap<String, String>>,
    /// Verify decode(encode(x)) against x (default true).
    pub verify: Option<bool>,
}

#[napi(object)]
pub struct EncodeResult {
    pub vtok: String,
    /// `{original: alias}`
    pub table: HashMap<String, String>,
    /// `{placeholder: hidden literal}`; pass back to `decode` (never shown to the model).
    pub literals: HashMap<String, String>,
}

#[napi(object)]
pub struct ChunkInfo {
    pub id: String,
    pub label: String,
    pub start: u32,
    pub end: u32,
}

#[napi(object)]
pub struct SplitResult {
    /// Skeleton + chunk map (or the full view when the file is not split).
    pub index: String,
    pub whole_file: bool,
    pub table: HashMap<String, String>,
    pub chunks: Vec<ChunkInfo>,
}

#[napi]
pub fn languages() -> Vec<LanguageInfo> {
    api::languages().iter().map(|l| LanguageInfo { name: l.name().to_string(), extensions: l.extensions().iter().map(|e| e.to_string()).collect() }).collect()
}

#[napi]
pub fn grammar(lang: String) -> Result<String> { Ok(api::language(&lang).map_err(err)?.grammar_prompt().to_string()) }

#[napi]
pub fn ntok(text: String) -> u32 { api::ntok(&text) as u32 }

#[napi]
pub fn encode(lang: String, source: String, options: Option<EncodeOptions>) -> Result<EncodeResult> {
    let o = options.unwrap_or(EncodeOptions { alias: None, view: None, hide_literals: None, table: None, verify: None });
    let opts = api::Options {
        alias: o.alias, view: o.view, hide_literals: o.hide_literals.map(|n| n as usize), verify: o.verify,
        table: o.table.map(|m| m.into_iter().collect::<BTreeMap<_, _>>()),
    };
    let r = api::encode(&lang, &source, &opts).map_err(err)?;
    Ok(EncodeResult { vtok: r.vtok, table: r.table.into_iter().collect(), literals: r.literals.into_iter().collect() })
}

#[napi]
pub fn decode(lang: String, vtok: String, table: Option<HashMap<String, String>>, literals: Option<HashMap<String, String>>) -> Result<String> {
    api::decode(&lang, &vtok, &inv(&table), &literals.unwrap_or_default()).map_err(err)
}

#[napi]
pub fn split(lang: String, source: String, max_lines: Option<u32>) -> Result<SplitResult> {
    let r = api::split_file(&lang, &source, max_lines.unwrap_or(150) as usize).map_err(err)?;
    Ok(SplitResult {
        index: r.index, whole_file: r.whole_file, table: r.table.into_iter().collect(),
        chunks: r.chunks.into_iter().map(|c| ChunkInfo { id: c.id, label: c.label, start: c.start as u32, end: c.end as u32 }).collect(),
    })
}

#[napi]
pub fn chunk(lang: String, source: String, id: String, max_lines: Option<u32>) -> Result<String> {
    api::chunk_of(&lang, &source, &id, max_lines.unwrap_or(150) as usize).map_err(err)
}

/// Decode edited chunks (`{chunkId: vtok}`) and splice them into the original source.
#[napi]
pub fn apply(lang: String, source: String, edits: HashMap<String, String>, max_lines: Option<u32>, force: Option<bool>) -> Result<String> {
    api::apply_chunks(&lang, &source, &edits.into_iter().collect(), max_lines.unwrap_or(150) as usize, force.unwrap_or(false)).map_err(err)
}

/// One stable alias table for many files of one language.
#[napi]
pub fn project_table(lang: String, sources: Vec<String>) -> Result<HashMap<String, String>> {
    let refs: Vec<&str> = sources.iter().map(|s| s.as_str()).collect();
    Ok(api::project_table(&lang, &refs).map_err(err)?.into_iter().collect())
}

/// Run the `vtok` command line (arguments without the program name); returns the exit code.
#[napi]
pub fn run_cli(args: Vec<String>) -> i32 { vtok_cli::run(&args) }
