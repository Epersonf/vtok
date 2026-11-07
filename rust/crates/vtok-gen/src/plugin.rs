//! `Language` implementation shared by every grammar in this crate.
use crate::emit::Emit;
use crate::spec::Spec;
use crate::tree::*;
use std::collections::{BTreeMap, HashMap, HashSet};
use tree_sitter::Node;
use vtok_core::aliasing::plan_aliases;
use vtok_core::lang::{EncodeOpts, Encoded, View};
use vtok_core::language::{Analysis, Language, Unit};
use vtok_core::{Result, VtokError};

pub struct Gen(pub &'static Spec);

const SPLITTABLE: [&str; 8] = ["class", "interface", "struct", "enum", "record", "mixin", "extension", "namespace"];

fn prep(s: &str) -> String { s.replace("\r\n", "\n") }

fn err_at(src: &str, n: Node, what: &str) -> VtokError {
    let row = n.start_position().row;
    let near: String = src.lines().nth(row).unwrap_or("").trim().chars().take(60).collect();
    VtokError::at(format!("{what} near {near:?}"), row + 1)
}

fn name_of(n: Node, src: &[u8]) -> String {
    let k = n.kind();
    if ["import", "using", "include", "package"].iter().any(|s| k.contains(s)) { return "import".into(); }
    let mut cur = n;
    loop {
        if let Some(x) = cur.child_by_field_name("name") { return text(x, src).to_string(); }
        match cur.child_by_field_name("declarator") { Some(d) => cur = d, None => break }
    }
    let mut st = vec![n];
    while let Some(x) = st.pop() {
        if x.child_count() == 0 && (x.kind() == "identifier" || x.kind().ends_with("_identifier")) { return text(x, src).to_string(); }
        for c in children(x).into_iter().rev() { st.push(c); }
    }
    k.to_string()
}

impl Gen {
    /// Does `vt`, decoded, parse to the same tree as the original?
    fn check(&self, vt: &str, table: &BTreeMap<String, String>, root: Node, src: &[u8]) -> std::result::Result<(), VtokError> {
        let inv: HashMap<String, String> = table.iter().map(|(o, a)| (a.clone(), o.clone())).collect();
        let back = self.decode(vt, &inv, &HashMap::new())?;
        let (a, mut b) = (canon_of(self.0, &back).ok_or_else(|| VtokError::new("roundtrip verification failed (result does not parse)"))?, vec![]);
        canon(root, src, &mut b);
        if a == b { return Ok(()); }
        let i = a.iter().zip(b.iter()).position(|(x, y)| x != y).unwrap_or(0);
        let ctx = |v: &Vec<String>| v[i.saturating_sub(4)..(i + 3).min(v.len())].join(" ");
        Err(VtokError::new(format!("roundtrip verification failed (tree differs): got [{}] want [{}]", ctx(&a), ctx(&b))))
    }

    fn wrapped(&self, text: &str) -> (String, usize) {
        let (open, close) = self.0.wrap.unwrap_or(("", ""));
        (format!("{open}\n{text}\n{close}\n"), open.len() + 1)
    }

    fn members<'a>(&self, root: Node<'a>) -> Vec<Node<'a>> {
        named_children(root).into_iter().find(|c| !is_comment(*c)).and_then(|c| c.child_by_field_name("body")).map(named_children).unwrap_or_default()
    }

    fn keywords(&self) -> HashSet<String> {
        let l = (self.0.grammar)();
        (0..l.node_kind_count() as u16)
            .filter(|&i| !l.node_kind_is_named(i) && l.node_kind_is_visible(i))
            .filter_map(|i| l.node_kind_for_id(i)).filter(|k| k.chars().all(is_word_char) && !k.is_empty()).map(String::from)
            .chain(self.0.words.split_whitespace().map(String::from)).collect()
    }

    fn push_units(&self, nodes: &[Node], src: &[u8], max_lines: usize, parent: Option<&str>, out: &mut Vec<Unit>) {
        for &n in nodes {
            if is_comment(n) || n.kind() == ";" { continue; }
            let (s, e) = (n.start_position().row + 1, n.end_position().row + 1);
            let name = name_of(n, src);
            if parent.is_none() && self.0.wrap.is_some() && e - s + 1 > max_lines && SPLITTABLE.iter().any(|k| n.kind().contains(k)) {
                let members: Vec<Node> = n.child_by_field_name("body").map(named_children).unwrap_or_default().into_iter().filter(|m| !is_comment(*m)).collect();
                if members.len() >= 2 { self.push_units(&members, src, max_lines, Some(&name), out); continue; }
            }
            out.push(Unit { name, kind: n.kind().into(), start: s, end: e, member: parent.is_some(), parent: parent.map(String::from), indent: n.start_position().column });
        }
    }

    fn tokens_of(&self, text: &str, member: bool) -> Option<Vec<(usize, usize)>> {
        let (w, off) = if member { self.wrapped(text) } else { (text.to_string(), 0) };
        let tree = parse(self.0, w.as_bytes());
        if first_error(tree.root_node()).is_some() { return None; }
        let mut out = vec![];
        let mut st = vec![tree.root_node()];
        while let Some(n) = st.pop() {
            if is_comment(n) { continue; }
            if n.child_count() == 0 {
                if n.end_byte() > n.start_byte() && n.start_byte() >= off && n.end_byte() - off <= text.len() { out.push((n.start_byte() - off, n.end_byte() - off)); }
            } else { for c in children(n).into_iter().rev() { st.push(c); } }
        }
        Some(out)
    }
}

impl Language for Gen {
    fn name(&self) -> &'static str { self.0.name }
    fn extensions(&self) -> &'static [&'static str] { self.0.exts }
    fn grammar_prompt(&self) -> &'static str { self.0.prompt }

    fn encode(&self, source: &str, opts: &EncodeOpts) -> Result<Encoded> {
        let source = prep(source);
        let src = source.as_bytes();
        let tree = parse(self.0, src);
        let root = tree.root_node();
        if let Some(e) = first_error(root) { return Err(err_at(&source, e, "source has syntax errors")); }
        let an = self.analyze(&source)?;
        let table: BTreeMap<String, String> = if !self.0.alias { BTreeMap::new() } else if let Some(t) = &opts.table { t.clone() } else if opts.alias {
            plan_aliases(&an.counts, &an.avoid, &an.reserved, &an.excluded, 2)
        } else { BTreeMap::new() };
        let tops: Vec<Node> = named_children(root);
        let skeleton = opts.view != View::Full;
        let none = BTreeMap::new();
        // aggressive spacing first, then keep source gaps, then both again without aliases (grammar ambiguities)
        let attempts: &[(bool, bool)] = if skeleton || !opts.verify { &[(false, true)] } else { &[(false, true), (true, true), (false, false), (true, false)] };
        let mut last = VtokError::new("roundtrip verification failed");
        for &(keep, aliased) in attempts {
            if !aliased && table.is_empty() { continue; }
            let tb = if aliased { &table } else { &none };
            let vt = Emit::new(src, self.0, tb, skeleton, keep, &tops).run(&children(root));
            if !opts.verify || skeleton { return Ok(Encoded { vtok: vt, table: tb.clone(), literals: BTreeMap::new() }); }
            match self.check(&vt, tb, root, src) {
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
        if let Some(e) = first_error(root) { return Err(err_at(&code, e, "syntax error")); }
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

    fn analyze(&self, source: &str) -> Result<Analysis> {
        let source = prep(source);
        let src = source.as_bytes();
        let tree = parse(self.0, src);
        let mut an = Analysis::default();
        an.avoid = self.keywords();
        import_names(tree.root_node(), src, &mut an.excluded, false);
        let mut st = vec![tree.root_node()];
        while let Some(n) = st.pop() {
            for c in children(n) { st.push(c); }
            if n.child_count() != 0 { continue; }
            let t = text(n, src);
            if t.chars().next().map_or(false, |c| c.is_alphabetic() || c == '_') && t.chars().all(is_word_char) { an.avoid.insert(t.to_string()); }
            if renamable(n, src) { *an.counts.entry(t.to_string()).or_insert(0) += 1; }
        }
        Ok(an)
    }

    fn units(&self, source: &str, max_lines: usize) -> Result<Vec<Unit>> {
        let source = prep(source);
        let tree = parse(self.0, source.as_bytes());
        let mut out = vec![];
        self.push_units(&named_children(tree.root_node()), source.as_bytes(), max_lines, None, &mut out);
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
        let none = HashMap::new();
        if !member { return self.decode(vtok, inv, &none); }
        let (open, close) = self.0.wrap.unwrap_or(("", ""));
        let out = self.decode(&format!("{open}\n{vtok}\n{close}\n"), inv, &none)?;
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
