//! Rust crates for graph_scope_rs.rs: each file's Cargo package (name, dependencies, root file), also when the
//! graphed directory is a crate's `src/` with the manifest above it.
use super::scope::{dir_of, normalize, Files, Scope};
use super::edges::Parsed;
use std::collections::HashMap;
use std::path::Path;

pub(crate) struct Crate { pub name: String, pub deps: Vec<String>, pub lib: Option<usize> }

fn cargo(root: &Path, dir: &str) -> Option<(String, Vec<String>)> {
    let t = std::fs::read_to_string(root.join(dir).join("Cargo.toml")).ok()?;
    let (mut name, mut deps, mut sect) = (String::new(), vec![], String::new());
    for l in t.lines().map(str::trim) {
        if l.starts_with('[') { sect = l.trim_matches(|c| c == '[' || c == ']').to_string(); continue; }
        let Some((k, v)) = l.split_once('=') else { continue };
        let k = k.trim().split('.').next().unwrap_or("").trim().trim_matches('"');
        if sect == "package" && k == "name" { name = v.trim().trim_matches('"').replace('-', "_"); }
        if sect.ends_with("dependencies") { deps.push(k.replace('-', "_")); }
    }
    Some((name, deps))
}

/// A crate root file under a manifest directory `d` that lies above the graphed root (`..`), as a graph path.
fn above(root: &Path, d: &str, f: &str, fs: &Files) -> Option<usize> {
    if !d.starts_with("..") { return None; }
    let r = std::fs::canonicalize(root).ok()?;
    let p = std::fs::canonicalize(root.join(d).join(f)).ok()?;
    fs.by_path.get(&super::edges::rel_of(&r, &p)).copied()
}

/// The crate of every Rust file (`sc[i].krate`, strict when known).
pub(crate) fn crates(root: &Path, parsed: &[Parsed], fs: &Files, sc: &mut [Scope], rs: &[usize]) -> Vec<Crate> {
    let mut crates: Vec<Crate> = vec![];
    let mut crate_of_dir: HashMap<String, Option<usize>> = HashMap::new();
    for &i in rs {
        let mut d = dir_of(&parsed[i].rel).to_string();
        let k = loop {
            if let Some(&k) = crate_of_dir.get(&d) { break k; }
            if let Some((name, deps)) = cargo(root, &d) {
                let lib = ["src/lib.rs", "src/main.rs"].iter().find_map(|f| fs.by_path.get(&normalize(&format!("{d}/{f}"))).copied().or_else(|| above(root, &d, f, fs)));
                crates.push(Crate { name, deps, lib });
                crate_of_dir.insert(d.clone(), Some(crates.len() - 1));
                break Some(crates.len() - 1);
            }
            // the graphed directory may be a crate's `src/`: look for the manifest a few levels above it
            if d.is_empty() || d.starts_with("..") {
                if d.matches("..").count() >= 3 { crate_of_dir.insert(d.clone(), None); break None; }
                d = if d.is_empty() { "..".into() } else { format!("{d}/..") };
                continue;
            }
            d = dir_of(&d).to_string();
        };
        sc[i].krate = k;
        sc[i].strict = k.is_some();
    }
    crates
}
