//! `Language` implementation shared by every grammar in this crate.
use crate::emit::Emit;
use crate::gen::{err_at, prep};
use crate::spec::Spec;
use crate::tree::*;
use std::collections::{BTreeMap, HashMap};
use tree_sitter::Node;
use vtok_core::aliasing::plan_aliases;
use vtok_core::lang::{EncodeOpts, Encoded, View};
use vtok_core::language::{Analysis, Language, Unit};
use vtok_core::{Result, VtokError};

pub struct Gen(pub &'static Spec);

impl Language for Gen {
    fn name(&self) -> &'static str { self.0.name }
    fn extensions(&self) -> &'static [&'static str] { self.0.exts }
    fn grammar_prompt(&self) -> &'static str { self.0.prompt }

    fn encode(&self, source: &str, opts: &EncodeOpts) -> Result<Encoded> {
        let source = prep(source);
        let src = source.as_bytes();
        let tree = parse(self.0, src);
        let root = tree.root_node();
        let plain = |vtok: String| Encoded { vtok, table: BTreeMap::new(), literals: BTreeMap::new() };
        if self.0.stmts && opts.view != View::Full { return Ok(plain(crate::markup::view(self.0, &source, root, opts.view).unwrap_or_default())); }
        if self.0.stmts && root.has_error() {
            return crate::stmts::encode_file(self.0, &source, false, opts.verify).map(plain).map_err(VtokError::new);
        }
        // unparsed regions are carried verbatim; a file that is mostly unparseable is refused
        let opaque = opaque_bytes(root);
        if let Some(e) = first_error(root).filter(|_| opaque * 2 > source.trim().len().max(1)) { return Err(err_at(&source, e, "source has syntax errors")); }
        if let Some(v) = crate::markup::view(self.0, &source, root, opts.view) { return Ok(plain(v)); }
        let an = self.analyze(&source)?;
        let table: BTreeMap<String, String> = if !self.0.alias { BTreeMap::new() } else if let Some(t) = &opts.table { t.clone() } else if opts.alias {
            plan_aliases(&an.counts, &an.avoid, &an.reserved, &an.excluded, 2)
        } else { BTreeMap::new() };
        let tops: Vec<Node> = crate::gen::items(root, self.0.dir);
        let skeleton = opts.view != View::Full;
        let none = BTreeMap::new();
        // aggressive spacing first, then keep source gaps, then both again without aliases (grammar ambiguities); with
        // unparsed regions, each again copying those regions whole (`coarse`)
        let errs = root.has_error();
        let attempts: Vec<(bool, bool, bool)> = if skeleton || !opts.verify { vec![(false, true, false)] } else {
            [true, false].into_iter().flat_map(|a| [false, true].into_iter().flat_map(move |c| [(false, a, c), (true, a, c)])).filter(|t| errs || !t.2).collect()
        };
        let mut last = VtokError::new("roundtrip verification failed");
        let protect = if errs { crate::lex::protected(self.0, &source) } else { vec![] };
        for (keep, aliased, coarse) in attempts {
            if !aliased && table.is_empty() { continue; }
            let tb = if aliased { &table } else { &none };
            let mut em = Emit::new(src, self.0, tb, skeleton, keep, &tops);
            em.coarse = coarse;
            em.protect = protect.clone();
            let vt = em.run(&children(root));
            if !opts.verify || skeleton { return Ok(Encoded { vtok: vt, table: tb.clone(), literals: BTreeMap::new() }); }
            match self.check(&vt, tb, root, src, errs) {
                Ok(()) => return Ok(Encoded { vtok: vt, table: tb.clone(), literals: BTreeMap::new() }),
                Err(e) => last = e,
            }
        }
        Err(last)
    }

    fn decode(&self, vtok: &str, table: &HashMap<String, String>, _literals: &HashMap<String, String>) -> Result<String> {
        let mut code = vtok.to_string();
        if !code.ends_with('\n') { code.push('\n'); }
        let tree = parse(self.0, code.as_bytes());
        let root = tree.root_node();
        // regions that do not parse are verbatim source text (see `encode`): they are passed through unchanged
        if table.is_empty() { return Ok(code); }
        let raw = code.as_bytes();
        let mut spans: Vec<(usize, usize, String)> = vec![];
        let mut st = vec![root];
        while let Some(n) = st.pop() {
            for c in children(n) { st.push(c); }
            if renamable(n, raw) { if let Some(o) = table.get(text(n, raw)) { spans.push((n.start_byte(), n.end_byte(), o.clone())); } }
        }
        spans.sort_by(|a, b| b.0.cmp(&a.0));
        let mut out = raw.to_vec();
        for (a, b, r) in spans { out.splice(a..b, r.into_bytes()); }
        Ok(String::from_utf8_lossy(&out).into_owned())
    }

    fn analyze(&self, source: &str) -> Result<Analysis> { Ok(self.analysis(&prep(source))) }

    fn units(&self, source: &str, max_lines: usize) -> Result<Vec<Unit>> {
        let source = prep(source);
        let tree = parse(self.0, source.as_bytes());
        if let Some(u) = crate::markup::units(self.0, &source, tree.root_node(), max_lines) { return Ok(u); }
        let mut out = vec![];
        self.push_units(&crate::gen::items(tree.root_node(), self.0.dir), source.as_bytes(), max_lines, None, &mut out);
        Ok(out)
    }

    fn encode_chunk(&self, text: &str, table: &BTreeMap<String, String>, member: bool) -> Result<String> {
        if !member { return Ok(self.encode(text, &EncodeOpts { table: Some(table.clone()), ..Default::default() })?.vtok); }
        let (w, _) = self.wrapped(text);
        let tree = parse(self.0, w.as_bytes());
        if let Some(e) = first_error(tree.root_node()) { return Err(err_at(&w, e, "source has syntax errors")); }
        let members = self.members(tree.root_node());
        Ok(Emit::new(w.as_bytes(), self.0, table, false, false, &members).run(&members))
    }

    fn decode_chunk(&self, vtok: &str, inv: &HashMap<String, String>, member: bool, indent: usize, _original: Option<&str>) -> Result<String> {
        // an edited chunk must parse (whole-file decoding passes unparsed regions through, edits do not)
        let none = HashMap::new();
        let (open, close) = if member { self.0.wrap.unwrap_or(("", "")) } else { ("", "") };
        let w = format!("{open}\n{vtok}\n{close}\n");
        if let Some(e) = first_error(parse(self.0, w.as_bytes()).root_node()) { return Err(err_at(&w, e, "syntax error")); }
        if !member { return self.decode(vtok, inv, &none); }
        let out = self.decode(&w, inv, &none)?;
        let tree = parse(self.0, out.as_bytes());
        let pad = " ".repeat(indent);
        let mut s = String::new();
        for m in self.members(tree.root_node()) {
            if is_comment(m) { continue; }
            s.push_str(&pad);
            s.push_str(text(m, out.as_bytes()));
            s.push('\n');
        }
        Ok(s)
    }

    fn tokens(&self, text: &str, member: bool) -> Option<Vec<(usize, usize)>> { self.tokens_of(text, member) }

    fn equivalent(&self, a: &str, b: &str, member: bool) -> bool {
        let w = |s: &str| if member { self.wrapped(s).0 } else { s.to_string() };
        match (canon_of(self.0, &w(a)), canon_of(self.0, &w(b))) { (Some(x), Some(y)) => x == y, _ => false }
    }

    fn validate(&self, source: &str) -> Result<()> {
        let tree = parse(self.0, source.as_bytes());
        match first_error(tree.root_node()) {
            Some(e) => Err(VtokError::at(format!("result is not valid {}", self.0.name), e.start_position().row + 1)),
            None => Ok(()),
        }
    }
}
