//! Symbol nodes and call edges for `vtok graph`. A callee name with one definition gives an EXTRACTED edge; with several
//! homonyms the edge is kept (INFERRED) only when evidence picks exactly one: the using file imports the defining
//! module, shares its directory, defines it itself, or the receiver names the owning type (`repo.save` -> `Repo.save`).
use super::{Graph, Node, EXTRACTED, INFERRED};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;
use vtok_api::*;

pub(crate) struct Parsed { pub rel: String, pub src: String, pub syms: Vec<(String, usize, usize)>, pub node_of: Vec<Option<usize>> }
pub(crate) type EdgeMap = BTreeMap<(usize, usize), u8>;
const MAX_CANDIDATES: usize = 40;

pub(crate) fn put(m: &mut EdgeMap, k: (usize, usize), c: u8) { let e = m.entry(k).or_insert(c); if c < *e { *e = c; } }

fn kind_of(text: &str, id: &str) -> &'static str {
    let w: Vec<&str> = text.split(|c: char| c.is_whitespace() || c == '(' || c == '<').filter(|x| !x.is_empty()).collect();
    if w.iter().take(5).any(|t| matches!(*t, "fn" | "def" | "function" | "func")) { if id.contains('.') { "method" } else { "function" } }
    else if w.iter().take(5).any(|t| matches!(*t, "class" | "struct" | "enum" | "trait" | "interface" | "type" | "union")) { "type" }
    else if id.contains('.') { "method" } else { "value" }
}

pub(crate) fn rel_of(root: &Path, p: &Path) -> String { p.strip_prefix(root).unwrap_or(p).display().to_string().replace(std::path::MAIN_SEPARATOR, "/") }

/// Parse every source file and add its symbols as nodes.
pub(crate) fn parse_files(root: &Path, g: &mut Graph) -> Vec<Parsed> {
    let mut files = vec![];
    crate::refs::walk(root, &mut files);
    let mut parsed = vec![];
    for f in &files {
        let Ok(src) = std::fs::read_to_string(f) else { continue };
        let rel = rel_of(root, f);
        let Ok(l) = language_for_path(&rel) else { continue };
        let Ok(syms) = symbols(l.name(), &src) else { continue };
        let lines: Vec<&str> = src.lines().collect();
        let mut node_of = vec![];
        for (id, a, b) in &syms {
            if id == "use" || id.starts_with("use#") || id.starts_with("import") || id.starts_with("mod#") { node_of.push(None); continue; }
            let text = lines.get(a - 1).copied().unwrap_or("");
            let t = text.trim();
            if (t.starts_with("mod ") || t.starts_with("pub mod ")) && t.ends_with(';') { node_of.push(None); continue; }
            g.nodes.push(Node { file: rel.clone(), name: id.split('#').next().unwrap_or(id).to_string(), a: *a, b: *b, kind: kind_of(text, id) });
            node_of.push(Some(g.nodes.len() - 1));
        }
        parsed.push(Parsed { rel, src, syms, node_of });
    }
    parsed
}

pub(crate) fn by_base(nodes: &[Node]) -> HashMap<String, Vec<usize>> {
    let mut m: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, n) in nodes.iter().enumerate() { if n.is_code() { m.entry(crate::refs::base(&n.name).to_string()).or_default().push(i); } }
    m
}

/// Identifier words of the import-like lines of a file.
fn import_words(src: &str) -> HashSet<String> {
    const KW: &[&str] = &["import ", "from ", "use ", "pub use ", "require", "#include", "using ", "package ", "mod ", "pub mod ", "@import", "export ", "include "];
    let mut out = HashSet::new();
    for l in src.lines() {
        let t = l.trim_start();
        if KW.iter().any(|k| t.starts_with(k)) || l.contains("require(") || l.contains(" from '") || l.contains(" from \"") || l.contains("import(") {
            for w in l.split(|c: char| !(c.is_alphanumeric() || c == '_')).filter(|w| !w.is_empty()) { out.insert(w.to_string()); }
        }
    }
    out
}

fn stem_of(file: &str) -> &str { let n = file.rsplit('/').next().unwrap_or(file); n.split('.').next().unwrap_or(n) }
fn dir_of(file: &str) -> &str { file.rsplit_once('/').map_or("", |x| x.0) }
fn dir_name(file: &str) -> &str { dir_of(file).rsplit('/').next().unwrap_or("") }
fn owner_of(name: &str) -> &str { name.rsplit_once('.').map_or("", |x| x.0.rsplit('.').next().unwrap_or(x.0)) }

/// The identifier right before `.name` or `::name` on this line.
fn receiver(line: &str, name: &str) -> Option<String> {
    let mut from = 0;
    while let Some(i) = line.get(from..).and_then(|t| t.find(name)) {
        let p = from + i;
        from = p + name.len();
        let head = line[..p].trim_end();
        let Some(head) = head.strip_suffix("::").or_else(|| head.strip_suffix('.')) else { continue };
        let r: String = head.chars().rev().take_while(|c| c.is_alphanumeric() || *c == '_').collect::<Vec<_>>().into_iter().rev().collect();
        if !r.is_empty() { return Some(r); }
    }
    None
}

fn score(nodes: &[Node], f: &Parsed, from: usize, to: usize, words: &HashSet<String>, recv: Option<&str>) -> u32 {
    let t = &nodes[to];
    let mut s = 0;
    if t.file == f.rel { s += 4; }
    if let Some(r) = recv {
        let owner = owner_of(&t.name);
        let own_self = matches!(r, "self" | "this" | "cls" | "Self" | "super") && !owner.is_empty() && owner == owner_of(&nodes[from].name);
        if own_self || (!owner.is_empty() && r.eq_ignore_ascii_case(owner)) || r == stem_of(&t.file) || r == dir_name(&t.file) { s += 3; }
    }
    if words.contains(stem_of(&t.file)) || (!dir_name(&t.file).is_empty() && words.contains(dir_name(&t.file))) { s += 2; }
    if dir_of(&t.file) == dir_of(&f.rel) { s += 1; }
    s
}

pub(crate) fn resolve(nodes: &[Node], parsed: &[Parsed], by_base: &HashMap<String, Vec<usize>>) -> EdgeMap {
    let mut m = EdgeMap::new();
    for f in parsed {
        let Ok(l) = language_for_path(&f.rel) else { continue };
        let Ok(ids) = idents_dot(l.name(), &f.src) else { continue };
        let local: HashSet<&str> = f.node_of.iter().flatten().map(|&i| crate::refs::base(&nodes[i].name)).collect();
        let words = import_words(&f.src);
        let lines: Vec<&str> = f.src.lines().collect();
        for (name, ln, dot, call) in ids {
            let Some(defs) = by_base.get(&name) else { continue };
            if defs.len() > MAX_CANDIDATES { continue; }
            // innermost listed symbol that contains the line
            let Some((idx, _)) = f.syms.iter().enumerate().filter(|(i, (_, a, b))| f.node_of[*i].is_some() && *a <= ln && ln <= *b).min_by_key(|(_, (_, a, b))| b - a) else { continue };
            let from = f.node_of[idx].unwrap();
            // the usual name-based rules: methods need `.name`, functions need a call, others any mention
            let line = lines.get(ln - 1).copied().unwrap_or("");
            let recv = if dot || line.contains("::") { receiver(line, &name) } else { None };
            // `pkg.func()` is a qualified call when the receiver names the module that defines the function
            let qualified = |to: usize| recv.as_deref().map_or(false, |r| r == stem_of(&nodes[to].file) || r == dir_name(&nodes[to].file));
            let cands: Vec<usize> = defs.iter().copied().filter(|&to| !(to == from || (nodes[to].file == f.rel && nodes[to].a == ln)) && match nodes[to].kind { "method" => dot, "function" => call && (!dot || qualified(to)), _ => !dot || local.contains(name.as_str()) }).collect();
            if defs.len() == 1 { if let Some(&to) = cands.first() { put(&mut m, (from, to), EXTRACTED); } continue; }
            if cands.is_empty() { continue; }
            let sc: Vec<u32> = cands.iter().map(|&to| score(nodes, f, from, to, &words, recv.as_deref())).collect();
            let best = *sc.iter().max().unwrap();
            if best > 0 && sc.iter().filter(|&&s| s == best).count() == 1 { put(&mut m, (from, cands[sc.iter().position(|&s| s == best).unwrap()]), INFERRED); }
        }
    }
    m
}
