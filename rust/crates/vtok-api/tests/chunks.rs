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

/// Editing one item must leave the formatting and comments of the other items byte-identical.
#[test]
fn edit_keeps_untouched_items_verbatim() {
    let src = "// keep me\nuse std::fmt;\n\n/// doc\nfn a(x: u32) -> u32 {\n    x + 1 // inline\n}\n\nfn b() -> u32 {\n    2\n}\n";
    let s = split_file("rust", src, 100).unwrap();
    let vt = chunk_of("rust", src, "c1", 100).unwrap();
    let edited = vt.replace("x+1", "x+2");
    assert_ne!(vt, edited);
    let mut e = BTreeMap::new();
    e.insert("c1".to_string(), edited);
    let out = apply_chunks("rust", src, &e, 100, false).unwrap();
    assert!(out.starts_with("// keep me\nuse std::fmt;\n\n/// doc\nfn a(") || out.contains("// keep me\nuse std::fmt;"), "{out}");
    assert!(out.contains("fn b() -> u32 {\n    2\n}\n"), "{out}");
    assert!(out.contains("x + 2 // inline"), "token-level patch keeps spacing and comments: {out}");
    let _ = s;
}

/// A string with an escape followed by a word ("\nEstimate") used to make the token patch add a space inside the string, fail
/// the equivalence check and rewrite the whole function in its minified form.
#[test]
fn rust_edit_next_to_an_escape_in_a_string_stays_a_token_patch() {
    let src = "fn f(x: u32) {\n    let marker = 1;\n\n    println!(\"\\nEstimate: {}\", x + marker);\n}\n";
    let vt = chunk_of("rust", src, "f", 100).unwrap();
    let edited = vt.replace("=1;", "=2;");
    assert_ne!(vt, edited, "{vt}");
    let mut e = BTreeMap::new();
    e.insert("f".to_string(), edited);
    let out = apply_chunks("rust", src, &e, 100, false).unwrap();
    assert_eq!(out, src.replace("= 1;", "= 2;"));
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

#[test]
fn python_edits_keep_original_formatting_and_comments() {
    let src = "class A:\n    W: str = \" \\n\\t\"\n    X: str = \"a b\"\n    # note\n    Y: int = 3\n\n    def f(self, a):\n        # why\n        return a + 1  # tail\n";
    let mut e = BTreeMap::new();
    e.insert("A.W".to_string(), "W:str=\" \\t\"\n".to_string());
    assert_eq!(apply_chunks("python", src, &e, 100, false).unwrap(), src.replace("\" \\n\\t\"", "\" \\t\""), "whitespace inside strings");
    let mut e = BTreeMap::new();
    e.insert("A.X".to_string(), "X:str=\"a\"\n".to_string());
    let out = apply_chunks("python", src, &e, 100, false).unwrap();
    assert_eq!(out, src.replace("\"a b\"", "\"a\""), "only the changed token is rewritten");
    let mut e = BTreeMap::new();
    e.insert("A.f".to_string(), "def f(self,a):\n return a+2\n".to_string());
    let out = apply_chunks("python", src, &e, 100, false).unwrap();
    assert_eq!(out, src.replace("a + 1", "a + 2"), "comments inside the item survive a token edit");
}
