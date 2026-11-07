//! Document nodes of `vtok graph`: PDFs, images, video/audio (with sidecar subtitles and transcripts), then text
//! documents (md, txt, rst, adoc, html, docx) and the references that link them to each other and to code.
use super::edges::{put, rel_of, EdgeMap, Parsed};
use super::media::{image_info, duration_secs, AUDIO, IMAGE, VIDEO};
use super::{refs, text, Graph, Node, EXTRACTED};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

const SKIP_DIRS: &[&str] = &["target", "node_modules", "venv", "dist", "build", "__pycache__", "vendor", "site-packages"];
const MAX_ITEMS: usize = 4000;
pub(crate) const MAX_DOC_NODES: usize = 3000;
pub(crate) const MAX_TEXT: u64 = 1_000_000;
pub(crate) const TEXT: &[&str] = &["md", "txt", "rst", "adoc", "html", "htm", "docx"];
const SUB: &[&str] = &["srt", "vtt", "txt"];

pub(crate) struct Item { pub path: PathBuf, pub rel: String, pub ext: String, pub size: u64, pub modified: std::time::SystemTime }

fn walk(root: &Path, dir: &Path, out: &mut Vec<Item>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    let mut es: Vec<_> = rd.flatten().collect();
    es.sort_by_key(|e| e.file_name());
    for e in es {
        if out.len() >= MAX_ITEMS { return; }
        let name = e.file_name().to_string_lossy().to_string();
        let Ok(ft) = e.file_type() else { continue };
        if ft.is_dir() { if !name.starts_with('.') && !SKIP_DIRS.contains(&name.as_str()) { walk(root, &e.path(), out); } continue; }
        let ext = name.rsplit_once('.').map(|x| x.1.to_ascii_lowercase()).unwrap_or_default();
        let wanted = ["pdf", "srt", "vtt"].contains(&ext.as_str()) || TEXT.contains(&ext.as_str()) || [IMAGE, VIDEO, AUDIO].iter().any(|l| l.contains(&ext.as_str()));
        let Ok(md) = e.metadata() else { continue };
        if ft.is_file() && wanted { out.push(Item { path: e.path(), rel: rel_of(root, &e.path()), ext, size: md.len(), modified: md.modified().unwrap_or(std::time::UNIX_EPOCH) }); }
    }
}

pub(crate) fn collect(root: &Path) -> Vec<Item> { let mut v = vec![]; walk(root, root, &mut v); v }

pub(crate) fn newest(root: &Path) -> std::time::SystemTime { collect(root).iter().map(|i| i.modified).max().unwrap_or(std::time::UNIX_EPOCH) }

/// (directory, file name up to the first dot): `talks/intro.en.srt` pairs with `talks/intro.mp4`.
fn key(rel: &str) -> (String, String) {
    let (d, n) = rel.rsplit_once('/').unwrap_or(("", rel));
    (d.to_string(), n.split('.').next().unwrap_or(n).to_string())
}

pub(crate) fn add(root: &Path, g: &mut Graph, parsed: &[Parsed], by_base: &HashMap<String, Vec<usize>>, m: &mut EdgeMap) {
    let items = collect(root);
    let start = g.nodes.len();
    let mut idx: HashMap<String, Vec<usize>> = HashMap::new();
    let mut av: Vec<(usize, usize)> = vec![];
    for (k, it) in items.iter().enumerate() {
        if g.nodes.len() - start >= MAX_DOC_NODES { break; }
        let ext = it.ext.as_str();
        let id = if ext == "pdf" { Some(super::pdf::add(g, &it.rel, &it.path, it.size, by_base, m)) }
        else if IMAGE.contains(&ext) && it.size <= 50_000_000 {
            image_info(&it.path, ext).map(|(w, h, txt)| {
                g.nodes.push(Node { file: it.rel.clone(), name: it.rel.clone(), a: w, b: h, kind: "image" });
                let id = g.nodes.len() - 1;
                for to in text::names_in(&txt, &g.nodes, by_base) { put(m, (id, to), EXTRACTED); }
                id
            })
        } else if VIDEO.contains(&ext) || AUDIO.contains(&ext) {
            g.nodes.push(Node { file: it.rel.clone(), name: it.rel.clone(), a: duration_secs(&it.path, ext).unwrap_or(0) as usize, b: (it.size / 1024) as usize, kind: if VIDEO.contains(&ext) { "video" } else { "audio" } });
            av.push((k, g.nodes.len() - 1));
            Some(g.nodes.len() - 1)
        } else { None };
        if let Some(id) = id { idx.entry(it.rel.rsplit('/').next().unwrap_or(&it.rel).to_ascii_lowercase()).or_default().push(id); }
    }
    let used = sidecars(g, &items, &av, by_base, m);
    refs::text_docs(g, &items, &used, &idx, by_base, m);
    refs::code_refs(g, parsed, &idx, m);
}

/// Subtitle and transcript files sharing the stem of a video or audio file become doc nodes linked from it.
fn sidecars(g: &mut Graph, items: &[Item], av: &[(usize, usize)], by_base: &HashMap<String, Vec<usize>>, m: &mut EdgeMap) -> HashSet<usize> {
    let mut subs: HashMap<(String, String), Vec<usize>> = HashMap::new();
    for (j, s) in items.iter().enumerate() { if SUB.contains(&s.ext.as_str()) && s.size <= MAX_TEXT { subs.entry(key(&s.rel)).or_default().push(j); } }
    let mut used = HashSet::new();
    for &(k, node) in av {
        for &j in subs.get(&key(&items[k].rel)).map(|v| v.as_slice()).unwrap_or(&[]) {
            let s = &items[j];
            let Ok(raw) = std::fs::read_to_string(&s.path) else { continue };
            let body = if s.ext == "txt" { raw.clone() } else { text::subtitle_text(&raw) };
            g.nodes.push(Node { file: s.rel.clone(), name: s.rel.clone(), a: 1, b: body.lines().count().max(1), kind: "doc" });
            let id = g.nodes.len() - 1;
            put(m, (node, id), EXTRACTED);
            for to in text::names_in(&body, &g.nodes, by_base) { put(m, (id, to), EXTRACTED); }
            used.insert(j);
        }
    }
    used
}
