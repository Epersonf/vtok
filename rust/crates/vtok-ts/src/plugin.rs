//! `Language` implementation for TypeScript (chunking hooks included). The trait methods are thin: `units` (top-level
//! units and their names), `spans` (item and token spans) and `chunk` (member chunks) hold the work, and `wrap` is
//! the synthetic class that lets a bare member parse.
use crate::analysis::{analyze, words};
use crate::codec::{decode, encode};
use crate::tree::{first_error, parse};
use crate::{EncodeOpts, Encoded, RESERVED_WORDS};
use std::collections::{BTreeMap, HashMap};
use vtok_core::language::{Analysis as CoreAnalysis, Language, Unit};
use vtok_core::{Result, VtokError};

mod chunk;
mod spans;
mod units;
mod wrap;

pub struct TypeScript;

static PROMPT: &str = "Vtok-TS: minified TypeScript. One top-level statement per line, no comments, no optional spaces,\nno ';' before '}' or at line end. Valid TypeScript otherwise (braces kept).\nNames in the alias table (alias=original) are used shortened; new names are written in full.\nExample:\nexport function val(u:string,p:string):boolean{if(!u||!p)return false;for(const x of xs)use(x);return db.q(u,p)}\n";

impl Language for TypeScript {
    fn name(&self) -> &'static str { "typescript" }
    fn extensions(&self) -> &'static [&'static str] { &[".ts", ".mts", ".cts", ".js", ".mjs", ".cjs", ".tsx", ".jsx"] }
    fn grammar_prompt(&self) -> &'static str { PROMPT }

    fn encode(&self, source: &str, opts: &EncodeOpts) -> Result<Encoded> { encode(source, opts) }

    fn decode(&self, vtok: &str, table: &HashMap<String, String>, literals: &HashMap<String, String>) -> Result<String> {
        decode(vtok, table, literals)
    }

    fn analyze(&self, source: &str) -> Result<CoreAnalysis> {
        let source = source.replace("\r\n", "\n");
        let tree = parse(source.as_bytes());
        let a = analyze(tree.root_node(), source.as_bytes());
        Ok(CoreAnalysis { counts: a.counts, excluded: a.excl, avoid: a.avoid, reserved: words(RESERVED_WORDS).collect() })
    }

    fn units(&self, source: &str, max_lines: usize) -> Result<Vec<Unit>> { units::units(source, max_lines) }

    fn encode_chunk(&self, text: &str, table: &BTreeMap<String, String>, member: bool) -> Result<String> {
        chunk::encode_chunk(text, table, member)
    }

    fn decode_chunk(&self, vtok: &str, inv: &HashMap<String, String>, member: bool, indent: usize, original: Option<&str>) -> Result<String> {
        chunk::decode_chunk(vtok, inv, member, indent, original)
    }

    fn item_spans(&self, text: &str, member: bool) -> Option<Vec<(usize, usize)>> { spans::item_spans(text, member) }

    fn tokens(&self, text: &str, member: bool) -> Option<Vec<(usize, usize)>> { spans::tokens(text, member) }

    fn equivalent(&self, a: &str, b: &str, member: bool) -> bool { chunk::equivalent(a, b, member) }

    fn validate(&self, source: &str) -> Result<()> {
        let tree = parse(source.as_bytes());
        match first_error(tree.root_node()) {
            Some(e) => Err(VtokError::at(format!("result is not valid TypeScript near line {}", e.start_position().row + 1), e.start_position().row + 1)),
            None => Ok(()),
        }
    }
}
