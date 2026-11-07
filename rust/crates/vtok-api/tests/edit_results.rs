//! What an edit leaves in the file: untouched text byte-identical, only the changed tokens rewritten, a deleted item gone
//! together with its doc comment.
use std::collections::BTreeMap;
use vtok_api::*;

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

/// Deleting a symbol takes its doc comment and attribute along (left behind they would describe the next item) and leaves
/// one blank line; a plain comment stays.
#[test]
fn deleting_a_symbol_takes_its_docs_and_leaves_one_blank_line() {
    let src = "/// first\nfn a() -> u32 {\n    1\n}\n\n/// second\n/// more\n#[inline]\nfn b() -> u32 {\n    2\n}\n\n// note on c\n/// third\nfn c() -> u32 {\n    3\n}\n";
    let delete = |src: &str, id: &str| {
        let mut e = BTreeMap::new();
        e.insert(id.to_string(), String::new());
        apply_chunks("rust", src, &e, 100, false).unwrap()
    };
    assert_eq!(delete(src, "b"), "/// first\nfn a() -> u32 {\n    1\n}\n\n// note on c\n/// third\nfn c() -> u32 {\n    3\n}\n");
    assert_eq!(delete(src, "c"), "/// first\nfn a() -> u32 {\n    1\n}\n\n/// second\n/// more\n#[inline]\nfn b() -> u32 {\n    2\n}\n\n// note on c\n");
    let last = "fn a() -> u32 {\n    1\n}\n\n/// last\nfn b() -> u32 {\n    2\n}\n";
    assert_eq!(delete(last, "b"), "fn a() -> u32 {\n    1\n}\n", "no blank line left at the end");
}

/// The same inside a chunk that holds several items: dropping the item's line from the chunk deletes it with its docs.
#[test]
fn dropping_an_item_from_a_chunk_takes_its_docs_too() {
    let src = "use std::fmt;\n\n/// first\nfn a() -> u32 {\n    1\n}\n\n/// second\n#[inline]\nfn b() -> u32 {\n    2\n}\n\n/// third\nfn c() -> u32 {\n    3\n}\n";
    let vt = chunk_of("rust", src, "c1", 100).unwrap();
    let kept: Vec<&str> = vt.lines().filter(|l| !l.contains("fn b") && !l.contains("inline")).collect();
    assert!(kept.len() < vt.lines().count() && !kept.is_empty(), "{vt}");
    let mut e = BTreeMap::new();
    e.insert("c1".to_string(), kept.join("\n") + "\n");
    let out = apply_chunks("rust", src, &e, 100, false).unwrap();
    assert_eq!(out, "use std::fmt;\n\n/// first\nfn a() -> u32 {\n    1\n}\n\n/// third\nfn c() -> u32 {\n    3\n}\n");
}
