//! Helpers of the generic plugin: verification, member wrapping, keywords, units and token spans.
use crate::plugin::Gen;
use crate::tree::*;
use std::collections::{BTreeMap, HashMap, HashSet};
use tree_sitter::Node;
use vtok_core::language::{Analysis, Language, Unit};
use vtok_core::VtokError;

const SPLITTABLE: [&str; 8] = ["class", "interface", "struct", "enum", "record", "mixin", "extension", "namespace"];

pub fn prep(s: &str) -> String { s.replace("
", "
") }

/// The top-level items of a file, looking through conditional-compilation blocks and `extern "C" {}` (a C header is
/// often one big `#ifndef GUARD` block): each item gets its own outline entry and its own line.
pub fn items<'a>(root: Node<'a>, dir: bool) -> Vec<Node<'a>> {
    let mut out = vec![];
    for c in named_children(root) {
        let k = c.kind();
        if dir && (k.starts_with("preproc_if") || k.starts_with("preproc_el") || k == "linkage_specification" || k == "declaration_list") {
            let skip: Vec<usize> = ["name", "condition", "value"].iter().filter_map(|f| c.child_by_field_name(f)).map(|x| x.id()).collect();
            let body = if k == "linkage_specification" { c.child_by_field_name("body").into_iter().collect() } else { vec![c] };
            for b in body { out.extend(items(b, dir).into_iter().filter(|x| !skip.contains(&x.id()))); }
        } else { out.push(c); }
    }
    out
}

pub fn err_at(src: &str, n: Node, what: &str) -> VtokError {
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
    // reached a bare declarator (`int *count;`): that is the name, not the type before it
    if cur.id() != n.id() && cur.child_count() == 0 && cur.kind().ends_with("identifier") { return text(cur, src).to_string(); }
    // else the first identifier outside type annotations (Dart `const double x = 1;` is named x, not double)
    for exact in [true, false] {
        let mut st = vec![n];
        while let Some(x) = st.pop() {
            let is_id = if exact { x.kind() == "identifier" } else { x.kind().ends_with("_identifier") };
            if x.child_count() == 0 && is_id { return text(x, src).to_string(); }
            if exact && (x.kind() == "type" || x.kind().ends_with("_type")) && x.id() != n.id() { continue; }
            for c in children(x).into_iter().rev() { st.push(c); }
        }
    }
    k.to_string()
}

impl Gen {
    /// Rename counts and the names aliases must avoid. With unparsed regions every word of the file is avoided (they
    /// may hide words in tokens the grammar does not expose), so an alias never reads like a name in verbatim text.
    pub fn analysis(&self, source: &str) -> Analysis {
        let src = source.as_bytes();
        let tree = parse(self.0, src);
        let mut an = Analysis::default();
        an.avoid = self.keywords();
        import_names(tree.root_node(), src, &mut an.excluded, false);
        if tree.root_node().has_error() { an.avoid.extend(source.split(|c: char| !is_word_char(c)).filter(|w| !w.is_empty()).map(String::from)); }
        let mut st = vec![tree.root_node()];
        while let Some(n) = st.pop() {
            for c in children(n) { st.push(c); }
            if n.child_count() != 0 { continue; }
            let t = text(n, src);
            if t.chars().next().map_or(false, |c| c.is_alphabetic() || c == '_') && t.chars().all(is_word_char) { an.avoid.insert(t.to_string()); }
            if renamable(n, src) { *an.counts.entry(t.to_string()).or_insert(0) += 1; }
        }
        an
    }

    /// Does `vt`, decoded, parse to the same tree as the original? With unparsed regions (`flat`), the token
    /// sequences must match instead, and the files must agree lexically too (`lex::proof`: same string literals, same
    /// code outside strings and comments), which does not depend on how the grammar recovered from the errors.
    pub fn check(&self, vt: &str, table: &BTreeMap<String, String>, root: Node, src: &[u8], flat: bool) -> std::result::Result<(), VtokError> {
        let inv: HashMap<String, String> = table.iter().map(|(o, a)| (a.clone(), o.clone())).collect();
        let back = self.decode(vt, &inv, &HashMap::new())?;
        if flat && crate::lex::proof(self.0, &back) != crate::lex::proof(self.0, &String::from_utf8_lossy(src)) {
            return Err(VtokError::new("roundtrip verification failed (strings or code differ lexically)"));
        }
        let (a, mut b) = if flat {
            let t = parse(self.0, back.as_bytes());
            let mut v = vec![];
            dump(t.root_node(), back.as_bytes(), &mut v, true);
            (v, vec![])
        } else {
            (canon_of(self.0, &back).ok_or_else(|| VtokError::new("roundtrip verification failed (result does not parse)"))?, vec![])
        };
        dump(root, src, &mut b, flat);
        if a == b { return Ok(()); }
        let i = a.iter().zip(b.iter()).position(|(x, y)| x != y).unwrap_or(a.len().min(b.len()));
        let ctx = |v: &Vec<String>| v[i.saturating_sub(4).min(v.len())..(i + 3).min(v.len())].join(" ");
        Err(VtokError::new(format!("roundtrip verification failed (tree differs): got [{}] want [{}]", ctx(&a), ctx(&b))))
    }

    pub fn wrapped(&self, text: &str) -> (String, usize) {
        let (open, close) = self.0.wrap.unwrap_or(("", ""));
        (format!("{open}\n{text}\n{close}\n"), open.len() + 1)
    }

    pub fn members<'a>(&self, root: Node<'a>) -> Vec<Node<'a>> {
        named_children(root).into_iter().find(|c| !is_comment(*c)).and_then(|c| c.child_by_field_name("body")).map(named_children).unwrap_or_default()
    }

    pub fn keywords(&self) -> HashSet<String> {
        let l = (self.0.grammar)();
        (0..l.node_kind_count() as u16)
            .filter(|&i| !l.node_kind_is_named(i) && l.node_kind_is_visible(i))
            .filter_map(|i| l.node_kind_for_id(i)).filter(|k| k.chars().all(is_word_char) && !k.is_empty()).map(String::from)
            .chain(self.0.words.split_whitespace().map(String::from)).collect()
    }

    pub fn push_units(&self, nodes: &[Node], src: &[u8], max_lines: usize, parent: Option<&str>, out: &mut Vec<Unit>) {
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

    pub fn tokens_of(&self, text: &str, member: bool) -> Option<Vec<(usize, usize)>> {
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

