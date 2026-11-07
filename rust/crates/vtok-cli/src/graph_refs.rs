//! Text documents and file-name references: which documents and code files mention which images, PDFs and media.
use super::docs::{Item, MAX_DOC_NODES, MAX_TEXT, TEXT};
use super::edges::{put, EdgeMap, Parsed};
use super::{text, Graph, Node, EXTRACTED, INFERRED};
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

type Idx = HashMap<String, Vec<usize>>;

/// File names with a media extension mentioned in `raw`, resolved to node ids as (node, confidence, byte offset).
/// A name with one match is EXTRACTED; with several, a path-suffix match is EXTRACTED and a same-directory one INFERRED.
fn media_refs(raw: &str, from: &str, nodes: &[Node], idx: &Idx) -> Vec<(usize, u8, usize)> {
    static R: OnceLock<Regex> = OnceLock::new();
    let re = R.get_or_init(|| Regex::new(r"(?i)[A-Za-z0-9_@%+~./\\-]+\.(?:png|jpe?g|gif|webp|svg|pdf|mp4|mkv|webm|mov|mp3|wav)\b").unwrap());
    let mut out = vec![];
    for mt in re.find_iter(raw) {
        let norm = mt.as_str().replace('\\', "/").replace("%20", " ").to_ascii_lowercase();
        let norm = norm.trim_start_matches("./").trim_start_matches("../").to_string();
        let Some(c) = idx.get(norm.rsplit('/').next().unwrap_or(&norm)) else { continue };
        let pick = if c.len() == 1 { Some((c[0], EXTRACTED)) } else {
            let by_path: Vec<usize> = c.iter().copied().filter(|&i| nodes[i].file.to_ascii_lowercase().ends_with(&norm)).collect();
            let dir = from.rsplit_once('/').map_or("", |x| x.0);
            let near: Vec<usize> = c.iter().copied().filter(|&i| nodes[i].file.rsplit_once('/').map_or("", |x| x.0) == dir).collect();
            if by_path.len() == 1 { Some((by_path[0], EXTRACTED)) } else if near.len() == 1 { Some((near[0], INFERRED)) } else { None }
        };
        if let Some((id, cf)) = pick { out.push((id, cf, mt.start())); }
    }
    out
}

pub(crate) fn text_docs(g: &mut Graph, items: &[Item], used: &HashSet<usize>, idx: &Idx, by_base: &HashMap<String, Vec<usize>>, m: &mut EdgeMap) {
    let start = g.nodes.len();
    for (j, it) in items.iter().enumerate() {
        if !TEXT.contains(&it.ext.as_str()) || used.contains(&j) || g.nodes.len() - start >= MAX_DOC_NODES { continue; }
        if it.size > if it.ext == "docx" { 10_000_000 } else { MAX_TEXT } { continue; }
        let (raw, body) = if it.ext == "docx" { let Some(t) = text::docx_text(&it.path) else { continue }; (t.clone(), t) } else {
            let Ok(r) = std::fs::read_to_string(&it.path) else { continue };
            let b = if it.ext == "html" || it.ext == "htm" { text::strip_tags(&r) } else { r.clone() };
            (r, b)
        };
        let links = text::names_in(&body, &g.nodes, by_base);
        let media = media_refs(&raw, &it.rel, &g.nodes, idx);
        if links.is_empty() && media.is_empty() { continue; }
        g.nodes.push(Node { file: it.rel.clone(), name: it.rel.clone(), a: 1, b: body.lines().count().max(1), kind: "doc" });
        let id = g.nodes.len() - 1;
        for to in links { put(m, (id, to), EXTRACTED); }
        for (to, c, _) in media { put(m, (id, to), c); }
    }
}

/// Code files that mention an image or media file by name: linked from the enclosing symbol, or a file node otherwise.
pub(crate) fn code_refs(g: &mut Graph, parsed: &[Parsed], idx: &Idx, m: &mut EdgeMap) {
    if idx.is_empty() { return; }
    let mut file_node: HashMap<String, usize> = HashMap::new();
    for f in parsed {
        if f.rel.ends_with(".html") || f.rel.ends_with(".htm") { continue; }
        for (to, c, off) in media_refs(&f.src, &f.rel, &g.nodes, idx) {
            let ln = f.src[..off].matches('\n').count() + 1;
            let inner = f.syms.iter().enumerate().filter(|(i, (_, a, b))| f.node_of[*i].is_some() && *a <= ln && ln <= *b).min_by_key(|(_, (_, a, b))| b - a).and_then(|(i, _)| f.node_of[i]);
            let from = match inner { Some(i) => i, None => *file_node.entry(f.rel.clone()).or_insert_with(|| {
                g.nodes.push(Node { file: f.rel.clone(), name: f.rel.clone(), a: 1, b: f.src.lines().count().max(1), kind: "doc" });
                g.nodes.len() - 1
            }) };
            put(m, (from, to), c);
        }
    }
}
