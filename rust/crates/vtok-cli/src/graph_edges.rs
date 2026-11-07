//! Nodes of `vtok graph`: one file node per source file plus its symbols (functions, methods, types, values), and
//! the edge map shared by every relation builder. Resolution lives in graph_resolve.rs, structure in graph_rels.rs.
use super::{Graph, Node, EXTRACTED, MENTIONS};
use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use vtok_api::*;

pub(crate) struct Parsed {
    pub rel: String,
    pub src: String,
    pub lang: &'static str,
    pub syms: Vec<(String, usize, usize)>,
    pub node_of: Vec<Option<usize>>,
    pub file_node: usize,
    /// type nodes declared as `trait` / `interface` / `protocol`
    pub traits: Vec<usize>,
}

/// (from, to, relation) -> score in percent (100 = EXTRACTED).
pub(crate) type EdgeMap = BTreeMap<(usize, usize, u8), u8>;

pub(crate) fn put_rel(m: &mut EdgeMap, from: usize, to: usize, rel: u8, score: u8) {
    if from == to { return; }
    let e = m.entry((from, to, rel)).or_insert(score);
    if score > *e { *e = score; }
}

/// Document links (relation `mentions`): EXTRACTED or INFERRED (0.7).
pub(crate) fn put(m: &mut EdgeMap, k: (usize, usize), c: u8) { put_rel(m, k.0, k.1, MENTIONS, if c == EXTRACTED { 100 } else { 70 }); }

/// The declaration line of a symbol: its first line that is not an attribute, decorator, comment or blank.
pub(crate) fn decl_line<'a>(lines: &[&'a str], a: usize, b: usize) -> &'a str {
    (a..=b.min(a + 12)).filter_map(|i| lines.get(i - 1).copied()).find(|l| {
        let t = l.trim_start();
        !(t.is_empty() || t.starts_with("#[") || t.starts_with("#![") || t.starts_with("//") || t.starts_with('@') || t.starts_with("/*") || t.starts_with('*') || t.starts_with('#') && !t.starts_with("#define"))
    }).unwrap_or_else(|| lines.get(a.saturating_sub(1)).copied().unwrap_or(""))
}

fn words(text: &str) -> Vec<&str> { text.split(|c: char| c.is_whitespace() || c == '(' || c == '<' || c == ':').filter(|x| !x.is_empty()).take(6).collect() }

fn kind_of(text: &str, id: &str) -> &'static str {
    let w = words(text);
    if w.iter().any(|t| matches!(*t, "fn" | "def" | "function" | "func")) { if id.contains('.') { "method" } else { "function" } }
    else if w.iter().any(|t| matches!(*t, "class" | "struct" | "enum" | "trait" | "interface" | "type" | "union" | "protocol" | "record")) { "type" }
    else if w.iter().any(|t| matches!(*t, "const" | "static" | "let" | "var" | "val")) { "value" }
    else if id.contains('.') { "method" } else { "value" }
}

pub(crate) fn rel_of(root: &Path, p: &Path) -> String { p.strip_prefix(root).unwrap_or(p).display().to_string().replace(std::path::MAIN_SEPARATOR, "/") }

/// A file with more symbols than this is generated bindings or a constants table (windows-sys, winapi, libc's platform
/// tables): thousands of one-line items whose resolution costs the square of their count (`windows-sys` 0.61.2:
/// 18-19 s and a 20 MB graph). It stays a file node; its symbols and the references inside it are left out and listed
/// in `Graph::bulk`. Among 10,370 source files of a cargo registry, the largest that is not a binding, a generated
/// file or a table had 630 symbols.
pub(crate) const BULK_SYMBOLS: usize = 1000;

/// `VTOK_GRAPH_MAX_FILE_SYMBOLS`: a number replaces the limit, 0 turns the guard off, anything else keeps the default.
pub(crate) fn limit_from(v: Option<&str>) -> usize {
    match v.and_then(|s| s.trim().parse::<usize>().ok()) { Some(0) => usize::MAX, Some(n) => n, None => BULK_SYMBOLS }
}

/// Parse every source file and add its file node and symbol nodes.
pub(crate) fn parse_files(root: &Path, g: &mut Graph) -> Vec<Parsed> {
    let mut files = vec![];
    crate::refs::walk(root, &mut files);
    let limit = limit_from(std::env::var("VTOK_GRAPH_MAX_FILE_SYMBOLS").ok().as_deref());
    let mut parsed = vec![];
    for f in &files {
        let Ok(raw) = std::fs::read_to_string(f) else { continue };
        let src = normalize_newlines(&raw);
        let rel = rel_of(root, f);
        let Ok(l) = language_for_path(&rel) else { continue };
        let Ok(mut syms) = symbols(l.name(), &src) else { continue };
        if syms.len() > limit { g.bulk.push((rel.clone(), syms.len())); syms.clear(); }
        let lines: Vec<&str> = src.lines().collect();
        let mut node_of = vec![];
        let mut traits = vec![];
        for (id, a, b) in &syms {
            let base = crate::refs::base(id);
            if matches!(base, "use" | "attr" | "extern" | "imports") || base.starts_with("import") || id.starts_with("mod#") || base.ends_with('!') { node_of.push(None); continue; }
            let text = decl_line(&lines, *a, *b);
            let t = text.trim();
            if t.split_whitespace().any(|w| w == "mod") && t.ends_with(';') { node_of.push(None); continue; }
            let name = id.split('#').next().unwrap_or(id).to_string();
            let mut kind = kind_of(text, id);
            // `tests.helper` inside an inline `mod tests { .. }` is a function, not a method
            if kind == "method" {
                if let Some((owner, _)) = name.rsplit_once('.') {
                    let m = format!("mod {owner}");
                    if !owner.contains(' ') && lines.iter().any(|l| l.trim_start().trim_start_matches("pub ").starts_with(&m)) { kind = "function"; }
                }
            }
            g.nodes.push(Node { file: rel.clone(), name, a: *a, b: *b, kind });
            if kind == "type" && words(text).iter().any(|w| matches!(*w, "trait" | "interface" | "protocol")) { traits.push(g.nodes.len() - 1); }
            node_of.push(Some(g.nodes.len() - 1));
        }
        containers(g, &rel, &lines, &mut syms, &mut node_of, &mut traits);
        g.nodes.push(Node { file: rel.clone(), name: rel.clone(), a: 1, b: lines.len().max(1), kind: "file" });
        parsed.push(Parsed { rel, src, lang: l.name(), syms, node_of, file_node: g.nodes.len() - 1, traits });
    }
    parsed
}

/// Traits, classes and interfaces whose body the plugin split into members (`Speak.speak` without a `Speak` node)
/// get a type node from their declaration line to their last member.
fn containers(g: &mut Graph, rel: &str, lines: &[&str], syms: &mut Vec<(String, usize, usize)>, node_of: &mut Vec<Option<usize>>, traits: &mut Vec<usize>) {
    let start = node_of.iter().flatten().min().copied().unwrap_or(g.nodes.len());
    let mut owners: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    for n in &g.nodes[start..] {
        let Some((o, _)) = n.name.rsplit_once('.') else { continue };
        if o.contains(" for ") || o.contains(' ') { continue; }
        let e = owners.entry(o.to_string()).or_insert((n.a, n.b));
        e.0 = e.0.min(n.a);
        e.1 = e.1.max(n.b);
    }
    for (o, (a, b)) in owners {
        if g.nodes[start..].iter().any(|n| n.name == o) { continue; }
        let last = o.rsplit('.').next().unwrap_or(&o);
        let decl = (a.saturating_sub(60).max(1)..a).rev().find_map(|i| {
            let w: Vec<&str> = lines.get(i - 1)?.split(|c: char| !(c.is_alphanumeric() || c == '_')).filter(|w| !w.is_empty()).collect();
            w.windows(2).find(|p| p[1] == last && matches!(p[0], "trait" | "interface" | "class" | "struct" | "enum" | "protocol" | "object")).map(|p| (i, p[0]))
        });
        let Some((d, kw)) = decl else { continue };
        let end = (b + 1).min(lines.len().max(1));
        g.nodes.push(Node { file: rel.to_string(), name: o.clone(), a: d, b: end, kind: "type" });
        if matches!(kw, "trait" | "interface" | "protocol") { traits.push(g.nodes.len() - 1); }
        syms.push((o, d, end));
        node_of.push(Some(g.nodes.len() - 1));
    }
}

pub(crate) fn by_base(nodes: &[Node]) -> HashMap<String, Vec<usize>> {
    let mut m: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, n) in nodes.iter().enumerate() { if n.is_code() { m.entry(crate::refs::base(&n.name).to_string()).or_default().push(i); } }
    m
}

/// Innermost symbol node of a parsed file that contains `line`.
pub(crate) fn enclosing(f: &Parsed, line: usize) -> Option<usize> {
    f.syms.iter().enumerate().filter(|(i, (_, a, b))| f.node_of[*i].is_some() && *a <= line && line <= *b).min_by_key(|(_, (_, a, b))| b - a).and_then(|(i, _)| f.node_of[i])
}
