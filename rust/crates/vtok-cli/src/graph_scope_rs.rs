//! Rust scopes for `vtok graph`: crates (Cargo.toml name and dependencies), the module tree (`mod x;` with or without
//! `#[path]`), and `use` trees (nested braces, `self`, `super`, `crate`, globs, `as` aliases, `pub use` re-exports).
//! Files outside any crate get no tree and fall back to non-strict resolution.
use super::edges::Parsed;
use super::scope::{dir_of, normalize, stem_of, Files, Scope};
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::OnceLock;

/// `a::{b, c::d as e, self}` -> [(["a","b"], None), (["a","c","d"], Some("e")), (["a","self"], None)]; `*` stays a segment.
fn flatten(s: &str, prefix: &[String], out: &mut Vec<(Vec<String>, Option<String>)>) {
    let s = s.trim().trim_start_matches("::").trim();
    if let Some(k) = s.find('{') {
        let mut p = prefix.to_vec();
        p.extend(s[..k].split("::").map(str::trim).filter(|x| !x.is_empty()).map(String::from));
        let inner = &s[k + 1..s.rfind('}').unwrap_or(s.len())];
        let (mut depth, mut from) = (0, 0);
        for (i, c) in inner.char_indices() {
            match c { '{' => depth += 1, '}' => depth -= 1, ',' if depth == 0 => { flatten(&inner[from..i], &p, out); from = i + 1; } _ => {} }
        }
        flatten(&inner[from..], &p, out);
        return;
    }
    if s.is_empty() { return; }
    let (path, alias) = match s.split_once(" as ") { Some((a, b)) => (a, Some(b.trim().to_string())), None => (s, None) };
    let mut p = prefix.to_vec();
    p.extend(path.split("::").map(str::trim).filter(|x| !x.is_empty()).map(String::from));
    out.push((p, alias));
}

/// The module file a path leads to and how many segments it used: `crate::refs::base` -> (refs.rs, 2).
pub(crate) fn resolve_path(sc: &[Scope], f: usize, segs: &[String]) -> (Option<usize>, usize) {
    let Some(first) = segs.first() else { return (Some(f), 0) };
    let mut cur = match first.as_str() {
        "crate" => sc[f].root,
        "self" => Some(f),
        "super" => sc[f].parent,
        s => sc[f].mods.get(s).and_then(|v| if v.len() == 1 { Some(v[0]) } else { None }),
    };
    let Some(mut at) = cur else { return (None, 0) };
    let mut used = 1;
    for s in &segs[1..] {
        let next = if s == "super" { sc[at].parent } else { sc[at].children.get(s).copied() };
        match next { Some(n) => { at = n; used += 1; } None => break }
    }
    cur = Some(at);
    (cur, used)
}

pub(crate) fn build(root: &Path, parsed: &[Parsed], fs: &Files, sc: &mut [Scope]) {
    static MOD: OnceLock<Regex> = OnceLock::new();
    static USE: OnceLock<Regex> = OnceLock::new();
    let modre = MOD.get_or_init(|| Regex::new(r#"(?m)^[ \t]*((?:#\[[^\]]*\][ \t]*\n?[ \t]*)*)(?:pub(?:\([^)]*\))?\s+)?mod\s+(\w+)\s*;"#).unwrap());
    let usere = USE.get_or_init(|| Regex::new(r"(?m)^[ \t]*(pub(?:\([^)]*\))?\s+)?use\s+([^;]+);").unwrap());
    let rs: Vec<usize> = (0..parsed.len()).filter(|&i| parsed[i].lang == "rust").collect();
    let crates = super::scope_crates::crates(root, parsed, fs, sc, &rs);
    let by_name: HashMap<&str, usize> = crates.iter().enumerate().map(|(i, c)| (c.name.as_str(), i)).collect();
    // module tree
    for &i in &rs {
        let p = &parsed[i];
        let own = if matches!(p.rel.rsplit('/').next(), Some("mod.rs" | "lib.rs" | "main.rs")) { dir_of(&p.rel).to_string() } else { normalize(&format!("{}/{}", dir_of(&p.rel), stem_of(&p.rel))) };
        for c in modre.captures_iter(&p.src) {
            let name = c[2].to_string();
            let attr = c.get(1).map_or("", |m| m.as_str());
            let path = attr.split("path").nth(1).and_then(|t| t.split('"').nth(1));
            let cands: Vec<String> = match path {
                Some(pth) => vec![normalize(&format!("{}/{pth}", dir_of(&p.rel)))],
                None => vec![normalize(&format!("{own}/{name}.rs")), normalize(&format!("{own}/{name}/mod.rs"))],
            };
            if let Some(&j) = cands.iter().find_map(|f| fs.by_path.get(f)) { sc[i].children.insert(name.clone(), j); sc[j].parent = Some(i); }
        }
    }
    for &i in &rs {
        let mut r = i;
        let mut guard = 0;
        while let Some(p) = sc[r].parent { r = p; guard += 1; if guard > 64 { break; } }
        sc[i].root = Some(r);
        let kids: Vec<(String, usize)> = sc[i].children.iter().map(|(k, v)| (k.clone(), *v)).collect();
        for (k, v) in kids { sc[i].mods.insert(k, vec![v]); }
        if let Some(k) = sc[i].krate {
            let mut seen: HashSet<usize> = HashSet::from([k]);
            let mut st = vec![k];
            while let Some(c) = st.pop() { for d in &crates[c].deps { if let Some(&j) = by_name.get(d.as_str()) { if seen.insert(j) { st.push(j); } } } }
            for d in crates[k].deps.iter().chain(std::iter::once(&crates[k].name)) {
                if let Some(lib) = by_name.get(d.as_str()).and_then(|&j| crates[j].lib) { sc[i].mods.entry(d.clone()).or_insert(vec![lib]); }
            }
            sc[i].deps = seen;
        }
    }
    // use trees (two passes so `use` of a module imported by another `use` resolves)
    for _ in 0..2 {
        for &i in &rs {
            let mut items = vec![];
            for c in usere.captures_iter(&parsed[i].src) { let mut v = vec![]; flatten(&c[2], &[], &mut v); items.extend(v.into_iter().map(|x| (c.get(1).is_some(), x))); }
            for (public, (path, alias)) in items { add_use(sc, i, public, &path, alias); }
        }
    }
    for &i in &rs { for v in [&mut sc[i].globs, &mut sc[i].reexports] { v.sort_unstable(); v.dedup(); } }
}

fn add_use(sc: &mut [Scope], i: usize, public: bool, path: &[String], alias: Option<String>) {
    let Some(last) = path.last() else { return };
    let base = &path[..path.len() - 1];
    if last == "*" {
        if let (Some(f), n) = resolve_path(sc, i, base) { if n == base.len() && f != i { sc[i].globs.push(f); if public { sc[i].reexports.push(f); } } }
        return;
    }
    let (full, local) = if last == "self" { (base, alias.unwrap_or_else(|| base.last().cloned().unwrap_or_default())) } else { (path, alias.unwrap_or_else(|| last.clone())) };
    if let (Some(f), n) = resolve_path(sc, i, full) { if n == full.len() && n > 0 { sc[i].mods.insert(local, vec![f]); return; } }
    if last == "self" { return; }
    let (f, n) = resolve_path(sc, i, base);
    let files = match f { Some(f) if n >= 1 && !base.is_empty() => vec![f], _ => vec![] };
    if public { sc[i].pub_names.insert(local.clone()); }
    sc[i].names.insert(local, (last.clone(), files));
}
