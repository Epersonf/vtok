//! SQL files the grammar cannot parse as a whole (Postgres DDL it lacks: row level security, policies, grants...):
//! the file is split into top-level statements and each one is parsed and minified on its own, so an unparsed
//! statement is carried as a verbatim chunk and the others are still compacted. Each statement is verified alone;
//! a join after a statement that ends in a verbatim chunk copies the source text exactly, so a split inside a string
//! the splitter misread can never change the text; the whole output is also checked lexically (`lex::proof`).
use crate::emit::Emit;
use crate::spec::Spec;
use crate::tree::*;
use std::collections::BTreeMap;

fn skip_to(b: &[u8], from: usize, pat: &[u8]) -> usize {
    (from..b.len()).find(|&j| b[j..].starts_with(pat)).map_or(b.len(), |j| j + pat.len())
}

/// Byte ranges of the top-level statements: each ends after a `;` outside strings, quoted names, comments and
/// dollar-quoted bodies; text after the last `;` is a last statement when it is not blank.
pub fn statements(src: &str) -> Vec<(usize, usize)> {
    let b = src.as_bytes();
    let (mut out, mut start, mut i) = (vec![], 0, 0);
    while i < b.len() {
        let c = b[i];
        i = match c {
            b'\'' | b'"' | b'`' => {
                let mut j = i + 1;
                while j < b.len() && !(b[j] == c && b.get(j + 1) != Some(&c)) { j += if b[j] == c { 2 } else { 1 }; }
                j + 1
            }
            b'-' if b.get(i + 1) == Some(&b'-') => skip_to(b, i, b"\n"),
            b'/' if b.get(i + 1) == Some(&b'*') => skip_to(b, i + 2, b"*/"),
            b'$' if i == 0 || !is_word_char(b[i - 1] as char) => {
                let tag = (i + 1..b.len()).find(|&j| !(b[j].is_ascii_alphanumeric() || b[j] == b'_')).filter(|&j| b[j] == b'$');
                match tag { Some(j) => skip_to(b, j + 1, &b[i..=j]), None => i + 1 }
            }
            b';' => { out.push((start, i + 1)); start = i + 1; i + 1 }
            _ => i + 1,
        };
    }
    if !src[start.min(src.len())..].trim().is_empty() { out.push((start, src.len())); }
    out
}

/// One statement's minified text with its source range; `vt` is where it sits in the output.
pub struct Piece { pub src: (usize, usize), pub vt: (usize, usize), pub opaque: usize }

pub fn encode(spec: &'static Spec, source: &str, skeleton: bool, keep: bool, coarse: bool) -> (String, Vec<Piece>) {
    let none = BTreeMap::new();
    let (mut out, mut pieces, mut prev_raw) = (String::new(), vec![], false);
    for (a, b) in statements(source) {
        let seg = &source[a..b];
        let tree = parse(spec, seg.as_bytes());
        let root = tree.root_node();
        let mut em = Emit::new(seg.as_bytes(), spec, &none, skeleton, keep, &named_children(root));
        em.coarse = coarse;
        if root.has_error() { em.protect = crate::lex::protected(spec, seg); }
        let (t, first, _, last_raw) = em.run_seg(&children(root));
        if t.is_empty() { continue; }
        // after a statement that ended cleanly no string can be open, so a line break is a safe join
        if !out.is_empty() { if prev_raw { out.push_str(&seg[..first]); } else { out.push('\n'); } }
        pieces.push(Piece { src: (a, b), vt: (out.len(), out.len() + t.len()), opaque: opaque_bytes(root) });
        out.push_str(&t);
        prev_raw = last_raw;
    }
    if !out.ends_with('\n') { out.push('\n'); }
    (out, pieces)
}

/// Every statement of the output parses to the same tokens as its source statement, and the whole output agrees with
/// the source lexically (SQL has no aliases: the output is its own decoding).
pub fn verify_pieces(spec: &Spec, source: &str, vt: &str, pieces: &[Piece]) -> Result<(), String> {
    if crate::lex::proof(spec, source) != crate::lex::proof(spec, vt) { return Err("roundtrip verification failed (strings or code differ lexically)".into()); }
    let flat = |s: &str| { let t = parse(spec, s.as_bytes()); let mut v = vec![]; dump(t.root_node(), s.as_bytes(), &mut v, true); v };
    for p in pieces {
        let (want, got) = (flat(&source[p.src.0..p.src.1]), flat(&vt[p.vt.0..p.vt.1]));
        if want != got {
            let i = want.iter().zip(&got).position(|(x, y)| x != y).unwrap_or(want.len().min(got.len()));
            return Err(format!("roundtrip verification failed (statement differs): got {:?} want {:?}", got.get(i), want.get(i)));
        }
    }
    Ok(())
}

/// The whole file statement by statement: tight spacing first, then source gaps kept; refused when most of it does
/// not parse or no attempt verifies.
pub fn encode_file(spec: &'static Spec, source: &str, skeleton: bool, verify: bool) -> Result<String, String> {
    let mut last = String::from("roundtrip verification failed");
    for (keep, coarse) in [(false, false), (true, false), (false, true), (true, true)] {
        let (vt, pieces) = encode(spec, source, skeleton, keep, coarse);
        if pieces.iter().map(|p| p.opaque).sum::<usize>() * 2 > source.trim().len().max(1) { return Err("source has syntax errors (mostly unparseable)".into()); }
        if !verify || skeleton { return Ok(vt); }
        match verify_pieces(spec, source, &vt, &pieces) { Ok(()) => return Ok(vt), Err(e) => last = e }
    }
    Err(last)
}
