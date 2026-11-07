//! `Language` implementation for Python (chunking hooks included). The trait methods are thin: `units` (top-level
//! units and their labels), `spans` (token and identifier spans) and `chunk` (member chunks) hold the work, and `wrap`
//! is the synthetic class that lets a bare member parse.
use crate::analysis::analyze;
use crate::codec::{decode, encode};
use crate::tree::{first_error, parse};
use crate::{EncodeOpts, Encoded};
use std::collections::{BTreeMap, HashMap, HashSet};
use vtok_core::language::{Analysis as CoreAnalysis, Language, Unit};
use vtok_core::{Result, VtokError};

mod chunk;
mod spans;
mod units;
mod wrap;

pub struct Python;

static PROMPT: &str = "Vtok: Python with minimal whitespace. Valid Python otherwise: 1-space indent, no spaces around\noperators, a header may take one simple statement inline (`if x:return 1`), simple statements joined by ';'.\nNames in the alias table (alias=original) are used shortened; new names are written in full.\nExample:\nclass AuthMgr:\n def val(u,p)->bool:\n  if not u or not p:return False\n  return db.q(u,p)\n";

impl Language for Python {
    fn name(&self) -> &'static str { "python" }
    fn extensions(&self) -> &'static [&'static str] { &[".py", ".pyi"] }
    fn grammar_prompt(&self) -> &'static str { PROMPT }

    fn encode(&self, source: &str, opts: &EncodeOpts) -> Result<Encoded> { encode(source, opts) }

    fn decode(&self, vtok: &str, table: &HashMap<String, String>, literals: &HashMap<String, String>) -> Result<String> {
        decode(vtok, table, literals)
    }

    fn analyze(&self, source: &str) -> Result<CoreAnalysis> {
        let source = source.replace("\r\n", "\n");
        let tree = parse(source.as_bytes());
        if let Some(e) = first_error(tree.root_node()) {
            return Err(VtokError::at("source has syntax errors", e.start_position().row + 1));
        }
        let a = analyze(tree.root_node(), source.as_bytes());
        Ok(CoreAnalysis { counts: a.counts, excluded: a.excluded, avoid: a.avoid, reserved: HashSet::new() })
    }

    fn tokens(&self, text: &str, member: bool) -> Option<Vec<(usize, usize)>> { spans::tokens(text, member) }

    fn identifier_spans(&self, text: &str) -> Option<Vec<(usize, usize)>> { spans::identifier_spans(text) }

    fn units(&self, source: &str, max_lines: usize) -> Result<Vec<Unit>> { units::units(source, max_lines) }

    fn encode_chunk(&self, text: &str, table: &BTreeMap<String, String>, member: bool) -> Result<String> {
        chunk::encode_chunk(text, table, member)
    }

    fn decode_chunk(&self, vtok: &str, inv: &HashMap<String, String>, member: bool, indent: usize, original: Option<&str>) -> Result<String> {
        chunk::decode_chunk(vtok, inv, member, indent, original)
    }

    fn equivalent(&self, a: &str, b: &str, member: bool) -> bool { chunk::equivalent(a, b, member) }

    fn validate(&self, source: &str) -> Result<()> {
        let tree = parse(source.as_bytes());
        match first_error(tree.root_node()) {
            Some(e) => Err(VtokError::at("result is not valid Python", e.start_position().row + 1)),
            None => Ok(()),
        }
    }
}
