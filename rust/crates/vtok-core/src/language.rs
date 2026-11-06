//! The contract every language plugin implements.
use crate::lang::{EncodeOpts, Encoded};
use crate::Result;
use std::collections::{BTreeMap, HashMap, HashSet};

/// Per-file facts used to plan alias tables (also across many files: project tables).
#[derive(Default, Clone)]
pub struct Analysis {
    pub counts: HashMap<String, usize>,
    pub excluded: HashSet<String>,
    pub avoid: HashSet<String>,
    pub reserved: HashSet<String>,
}

/// An addressable region of a source file (1-based inclusive lines).
#[derive(Clone, Debug)]
pub struct Unit {
    pub name: String,
    pub kind: String,
    pub start: usize,
    pub end: usize,
    pub member: bool,
    pub parent: Option<String>,
    pub indent: usize,
}

pub trait Language: Sync {
    fn name(&self) -> &'static str;
    fn extensions(&self) -> &'static [&'static str];
    fn grammar_prompt(&self) -> &'static str;
    fn encode(&self, source: &str, opts: &EncodeOpts) -> Result<Encoded>;
    /// table: {alias: original}; literals: {placeholder: original text}
    fn decode(&self, vtok: &str, table: &HashMap<String, String>, literals: &HashMap<String, String>) -> Result<String>;
    fn analyze(&self, source: &str) -> Result<Analysis>;
    // ---- chunking ----
    fn units(&self, source: &str, max_lines: usize) -> Result<Vec<Unit>>;
    fn encode_chunk(&self, text: &str, table: &BTreeMap<String, String>, member: bool) -> Result<String>;
    fn decode_chunk(&self, vtok: &str, inv: &HashMap<String, String>, member: bool, indent: usize, original: Option<&str>) -> Result<String>;
    /// Byte spans (within `text`) of the top-level items, one per Vtok line of `encode_chunk`; None = unsupported.
    fn item_spans(&self, _text: &str, _member: bool) -> Option<Vec<(usize, usize)>> { None }
    /// Byte spans of the leaf tokens of `text` (comments excluded), in order; None = unsupported.
    fn tokens(&self, _text: &str, _member: bool) -> Option<Vec<(usize, usize)>> { None }
    fn equivalent(&self, a: &str, b: &str, member: bool) -> bool;
    fn validate(&self, source: &str) -> Result<()>;
}
