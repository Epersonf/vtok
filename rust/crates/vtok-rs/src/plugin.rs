//! `Language` implementation for Rust (chunking hooks included). The trait methods are thin: `units` (items and their
//! names), `spans` (item and token spans) and `chunk` (member chunks) hold the work, and `wrap` is the synthetic `mod`
//! that lets a bare member parse.
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

pub struct Rust;

static PROMPT: &str = "Vtok-Rust: minified Rust. One top-level item per line, no comments, no optional spaces.\nEvery ';' and ',' is kept. Valid Rust otherwise.\nNames in the alias table (alias=original) are used shortened; new names are written in full.\nExample:\npub fn val(u:&str,p:&str)->bool{if u.is_empty()||p.is_empty(){return false}db.q(u,p)}\n";

impl Language for Rust {
    fn name(&self) -> &'static str { "rust" }
    fn extensions(&self) -> &'static [&'static str] { &[".rs"] }
    fn grammar_prompt(&self) -> &'static str { PROMPT }

    fn encode(&self, source: &str, opts: &EncodeOpts) -> Result<Encoded> { encode(source, opts) }

    fn decode(&self, vtok: &str, table: &HashMap<String, String>, literals: &HashMap<String, String>) -> Result<String> {
        decode(vtok, table, literals)
    }

    fn analyze(&self, source: &str) -> Result<CoreAnalysis> {
        let source = source.replace("\r\n", "\n");
        let tree = parse(source.as_bytes());
        let a = analyze(tree.root_node(), source.as_bytes());
        Ok(CoreAnalysis { counts: a.counts, excluded: a.excl, avoid: a.avoid, reserved: HashSet::new() })
    }

    fn units(&self, source: &str, max_lines: usize) -> Result<Vec<Unit>> { units::units(source, max_lines) }

    fn encode_chunk(&self, text: &str, table: &BTreeMap<String, String>, member: bool) -> Result<String> {
        chunk::encode_chunk(text, table, member)
    }

    fn decode_chunk(&self, vtok: &str, inv: &HashMap<String, String>, member: bool, indent: usize, original: Option<&str>) -> Result<String> {
        chunk::decode_chunk(vtok, inv, member, indent, original)
    }

    fn item_spans(&self, text: &str, member: bool) -> Option<Vec<(usize, usize)>> { spans::item_spans(text, member) }

    fn identifier_spans(&self, text: &str) -> Option<Vec<(usize, usize)>> { Some(super::idents::spans(text)) }

    fn tokens(&self, text: &str, member: bool) -> Option<Vec<(usize, usize)>> { spans::tokens(text, member) }

    fn equivalent(&self, a: &str, b: &str, member: bool) -> bool { chunk::equivalent(a, b, member) }

    fn validate(&self, source: &str) -> Result<()> {
        let tree = parse(source.as_bytes());
        match first_error(tree.root_node()) {
            Some(e) => Err(VtokError::at(format!("result is not valid Rust near line {}", e.start_position().row + 1), e.start_position().row + 1)),
            None => Ok(()),
        }
    }
}
