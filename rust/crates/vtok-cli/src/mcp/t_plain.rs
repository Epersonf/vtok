//! The plain-text fallbacks: any file can be read and edited by exact text, and the compact form is used only when it
//! saves enough (`min-gain`).
use super::args::fnv;
use super::source::{tool_chunk, tool_edit, tool_read};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

fn dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("vtok-plain-{tag}-{}", std::process::id()));
    std::fs::remove_dir_all(&d).ok();
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn edit(file: &Path, rev: &str, replace: Value) -> Result<String, String> {
    tool_edit(&json!({"path": file.to_str().unwrap(), "rev": rev, "replace": replace}))
}

fn read(file: &Path) -> String {
    tool_read(&json!({"path": file.to_str().unwrap()})).unwrap()
}

#[test]
fn a_file_with_no_vtok_notation_is_read_as_text_with_its_rev() {
    let f = dir("md").join("notes.md");
    std::fs::write(&f, "# Title\n\nsome text\n").unwrap();
    let out = read(&f);
    assert!(out.contains(&format!("rev={}", fnv("# Title\n\nsome text\n"))) && out.ends_with("---\n# Title\n\nsome text\n"), "{out}");
}

#[test]
fn replace_edits_exact_text_in_any_file_and_keeps_crlf() {
    let f = dir("crlf").join("config.toml");
    std::fs::write(&f, "a = 1\r\nb = 2\r\n").unwrap();
    let out = edit(&f, &fnv("a = 1\nb = 2\n"), json!([{"old": "b = 2", "new": "b = 3\nc = 4"}])).unwrap();
    assert!(out.contains("written") && out.contains("+1 lines"), "{out}");
    assert_eq!(std::fs::read_to_string(&f).unwrap(), "a = 1\r\nb = 3\r\nc = 4\r\n");
}

#[test]
fn replace_refuses_missing_or_repeated_text_and_writes_nothing() {
    let f = dir("refuse").join("x.txt");
    std::fs::write(&f, "x\nx\ny\n").unwrap();
    let rev = fnv("x\nx\ny\n");
    assert!(edit(&f, &rev, json!([{"old": "x", "new": "z"}])).unwrap_err().contains("2 times"));
    assert!(edit(&f, &rev, json!([{"old": "nope", "new": "z"}])).unwrap_err().contains("not found"));
    // the first replacement would apply, the second does not: the file stays as it was
    assert!(edit(&f, &rev, json!([{"old": "y", "new": "w"}, {"old": "nope", "new": "q"}])).is_err());
    assert_eq!(std::fs::read_to_string(&f).unwrap(), "x\nx\ny\n");
    edit(&f, &rev, json!([{"old": "x", "new": "z", "all": true}])).unwrap();
    assert_eq!(std::fs::read_to_string(&f).unwrap(), "z\nz\ny\n");
}

#[test]
fn replace_checks_the_rev_and_a_dry_run_writes_nothing() {
    let f = dir("rev").join("x.md");
    std::fs::write(&f, "one\n").unwrap();
    assert!(edit(&f, "0000", json!([{"old": "one", "new": "two"}])).unwrap_err().contains("changed since"));
    let dry = json!({"path": f.to_str().unwrap(), "rev": fnv("one\n"), "dry_run": true, "replace": [{"old": "one", "new": "two"}]});
    assert!(tool_edit(&dry).unwrap().contains("dry run"));
    assert_eq!(std::fs::read_to_string(&f).unwrap(), "one\n");
}

#[test]
fn a_tiny_source_file_comes_back_as_text() {
    let f = dir("tiny").join("t.rs");
    std::fs::write(&f, "fn a() {}\n").unwrap();
    let out = read(&f);
    assert!(out.contains("plain text") && out.ends_with("---\nfn a() {}\n"), "{out}");
}

fn commented_module() -> String {
    (0..20).map(|i| format!("/// Adds {i} to the value and returns the sum; this documentation is long on purpose so that the comment dominates.\npub fn add_{i}(value: u64) -> u64 {{\n    value + {i}\n}}\n\n")).collect()
}

#[test]
fn min_gain_decides_between_the_compact_form_and_the_text() {
    let d = dir("gain");
    let f = d.join("m.rs");
    std::fs::write(&f, commented_module()).unwrap();
    assert!(read(&f).contains("lang=rust"), "a file made of comments saves far more than 10%");
    std::fs::write(d.join(".vtok.json"), r#"{"min_gain": 90}"#).unwrap();
    assert!(read(&f).contains("plain text"), "but not 90%");
}

#[test]
fn a_symbol_comes_back_as_its_lines_when_vtok_saves_less_than_min_gain() {
    let d = dir("chunk");
    let f = d.join("c.rs");
    let src = "pub fn small(v: u64) -> u64 { v + 1 }\n\npub fn commented(v: u64) -> u64 {\n    // this first comment line explains the step at great length so that dropping it matters a lot\n    // and a second comment line keeps going with more words to count as tokens in the plain text\n    let w = v + 1;\n    // a third comment line that says one more thing about the value\n    w * 2\n}\n";
    std::fs::write(&f, src).unwrap();
    let chunk = |id: &str| tool_chunk(&json!({"path": f.to_str().unwrap(), "id": id})).unwrap();
    assert!(!chunk("commented").contains("plain text"));
    std::fs::write(d.join(".vtok.json"), r#"{"min_gain": 90}"#).unwrap();
    let out = chunk("small");
    assert!(out.contains("lines 1-1 plain text") && out.ends_with("---\npub fn small(v: u64) -> u64 { v + 1 }"), "{out}");
}
