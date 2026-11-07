//! Chunking invariants over real corpora (set VTOK_PY_CORPUS / VTOK_TS_CORPUS to directories).
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use vtok_api::*;

fn files(dir: &Path, ext: &str, out: &mut Vec<PathBuf>) {
    let mut kids: Vec<PathBuf> = std::fs::read_dir(dir).unwrap().filter_map(|e| e.ok()).map(|e| e.path()).collect();
    kids.sort();
    for k in kids {
        if k.is_dir() { files(&k, ext, out); }
        else if k.to_string_lossy().ends_with(ext) && !k.to_string_lossy().ends_with(".d.ts") { out.push(k); }
    }
}

fn check(lang: &str, dir: &str, ext: &str, max_lines: usize) {
    let l = language(lang).unwrap();
    let mut fs = vec![];
    files(Path::new(dir), ext, &mut fs);
    let (mut n_files, mut n_chunks, mut fail) = (0, 0, 0);
    for f in fs {
        let Ok(src) = std::fs::read_to_string(&f) else { continue };
        let src = normalize_newlines(&src);
        if src.lines().count() <= max_lines { continue; }
        let Ok(chunks) = split::plan_chunks(l, &src, max_lines) else { continue };
        let table = split::file_table(l, &src);
        let mut edits: BTreeMap<String, String> = BTreeMap::new();
        let mut ok = true;
        for c in &chunks {
            match split::chunk_vtok(l, &src, c, &table) {
                Ok(v) => { edits.insert(c.id.clone(), v); }
                Err(e) => { eprintln!("CHUNK ENCODE FAIL {} {}: {e}", f.display(), c.id); ok = false; }
            }
        }
        n_files += 1; n_chunks += chunks.len();
        if !ok { fail += 1; continue; }
        match split::apply_edits(l, &src, &chunks, &edits, &table, false) {
            Ok(out) if out == src => {}
            Ok(_) => { eprintln!("UNCHANGED NOT IDENTICAL {}", f.display()); fail += 1; continue; }
            Err(e) => { eprintln!("APPLY FAIL {}: {e}", f.display()); fail += 1; continue; }
        }
        match split::apply_edits(l, &src, &chunks, &edits, &table, true) {
            Ok(out) => if !l.equivalent(&src, &out, false) { eprintln!("FORCE NOT EQUIVALENT {}", f.display()); fail += 1; },
            Err(e) => { eprintln!("FORCE FAIL {}: {e}", f.display()); fail += 1; }
        }
    }
    eprintln!("{lang}: files={n_files} chunks={n_chunks} fail={fail}");
    assert_eq!(fail, 0);
    assert!(n_files > 0, "corpus empty");
}

#[test]
fn python_chunks() {
    if let Ok(d) = std::env::var("VTOK_PY_CORPUS") { check("python", &d, ".py", 60); }
}

#[test]
fn typescript_chunks() {
    if let Ok(d) = std::env::var("VTOK_TS_CORPUS") { check("typescript", &d, ".ts", 60); }
}

/// `imports` is what the outline calls a run of import lines, and it reads and edits like any other symbol.
#[test]
fn the_imports_the_outline_lists_are_addressable() {
    let src = "use std::fmt;\nuse std::io;\n\nfn a() -> u32 {\n    1\n}\n\nuse std::path::Path;\n\nfn b(_p: &Path) {}\n";
    let o = outline("rust", src).unwrap();
    assert_eq!(o, "imports L1-2\na L4-6\nimports#2 L8\nb L10\n");
    let vt = chunk_of("rust", src, "imports", 100).unwrap();
    assert!(vt.contains("use std::fmt;") && vt.contains("use std::io;") && !vt.contains("Path"), "{vt}");
    assert!(chunk_of("rust", src, "imports#2", 100).unwrap().contains("std::path::Path"));
    let mut e = BTreeMap::new();
    e.insert("imports".to_string(), vt.replace("std::io", "std::fs"));
    assert_eq!(apply_chunks("rust", src, &e, 100, false).unwrap(), src.replace("std::io", "std::fs"));
    let py = "import os\nimport sys\n\n\ndef f():\n    return os.name\n";
    assert_eq!(outline("python", py).unwrap(), "imports L1-2\nf L5-6\n");
    assert!(chunk_of("python", py, "imports", 100).unwrap().contains("import sys"));
}

/// A symbol name (or Class.method) works wherever a chunk id does.
#[test]
fn symbols_address_items() {
    let src = "struct S;

impl S {
    fn m(&self) -> u32 {
        1
    }
    fn n(&self) -> u32 {
        2
    }
}

fn top() -> u32 {
    3
}
";
    assert!(chunk_of("rust", src, "top", 100).unwrap().contains("fn top"));
    assert!(chunk_of("rust", src, "S.m", 100).unwrap().contains("fn m"));
    assert!(chunk_of("rust", src, "m", 100).unwrap().contains("fn m"));
    assert!(chunk_of("rust", src, "nope", 100).is_err());
    let mut e = BTreeMap::new();
    e.insert("n".to_string(), chunk_of("rust", src, "n", 100).unwrap().replace("{2}", "{9}"));
    let out = apply_chunks("rust", src, &e, 100, false).unwrap();
    assert_eq!(out, src.replace("        2
", "        9
"));
}
