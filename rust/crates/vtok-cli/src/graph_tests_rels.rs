//! Tests for relation kinds and scores on a small Rust crate, and for the false positives the scope rules remove.
use super::tests::{put, tmp};
use super::*;

pub(crate) fn rel(g: &Graph, a: usize, b: usize, r: u8) -> Option<u8> {
    (0..g.edges.len()).find(|&k| g.edges[k] == (a, b) && g.rel[k] == r).map(|k| g.score[k])
}
pub(crate) fn node(g: &Graph, name: &str) -> usize { g.nodes.iter().position(|n| n.name == name).unwrap_or_else(|| panic!("no node {name}")) }

fn krate(tag: &str) -> std::path::PathBuf {
    let d = tmp(tag);
    put(&d, "a/Cargo.toml", "[package]\nname = \"a\"\n");
    put(&d, "a/src/lib.rs", "mod speak;\nmod util;\npub use speak::Speak;\nuse util::helper;\n\npub const LIMIT: u8 = 3;\n\npub struct Dog { pub name: String }\n\nimpl Speak for Dog {\n    fn speak(&self) -> u8 { helper() + self.bark() }\n}\n\nimpl Dog {\n    pub fn bark(&self) -> u8 { LIMIT }\n    pub fn new() -> Dog { Dog { name: String::new() } }\n}\n\npub fn run(d: &Dog) -> u8 {\n    let text = 1;\n    let len = d.name.len();\n    let r = regex::Regex::new(\"x\");\n    let f = |x: u8| x + 1;\n    d.speak() + text + f(2) + len as u8 + crate::util::twice(1) + Dog::new().bark()\n}\n\n#[cfg(test)]\nmod tests {\n    use super::*;\n    #[test]\n    fn run_counts() { assert_eq!(run(&Dog::new()), 4); }\n}\n");
    put(&d, "a/src/speak.rs", "pub trait Speak {\n    fn speak(&self) -> u8;\n}\n");
    put(&d, "a/src/util.rs", "pub fn helper() -> u8 { 1 }\npub fn twice(x: u8) -> u8 { x * 2 }\npub fn text() -> u8 { 0 }\npub fn f(x: u8) -> u8 { x }\npub fn new() -> u8 { 9 }\n");
    d
}

#[test]
fn every_relation_kind_with_scores() {
    let d = krate("rels");
    let g = build(&d);
    let (run, dog, speak) = (node(&g, "run"), node(&g, "Dog"), node(&g, "Speak"));
    assert_eq!(rel(&g, dog, speak, IMPLEMENTS), Some(100), "impl Speak for Dog");
    assert_eq!(rel(&g, dog, node(&g, "Dog.bark"), CONTAINS), Some(100));
    assert_eq!(rel(&g, node(&g, "a/src/lib.rs"), dog, CONTAINS), Some(100));
    assert_eq!(rel(&g, node(&g, "a/src/lib.rs"), node(&g, "a/src/util.rs"), IMPORTS), Some(100));
    assert_eq!(rel(&g, run, dog, USES_TYPE), Some(100));
    assert_eq!(rel(&g, node(&g, "Dog.bark"), node(&g, "LIMIT"), REFERENCES), Some(100));
    assert_eq!(rel(&g, node(&g, "Speak for Dog.speak"), node(&g, "helper"), CALLS), Some(100), "imported by `use util::helper`");
    assert_eq!(rel(&g, node(&g, "Speak for Dog.speak"), node(&g, "Dog.bark"), CALLS), Some(100), "self.bark()");
    assert_eq!(rel(&g, run, node(&g, "twice"), CALLS), Some(100), "crate::util::twice");
    assert_eq!(rel(&g, run, node(&g, "Dog.new"), CALLS), Some(100), "Dog::new");
    assert_eq!(rel(&g, run, node(&g, "Speak for Dog.speak"), CALLS), Some(90), "d: &Dog types the receiver");
    assert_eq!(rel(&g, node(&g, "tests.run_counts"), run, TESTS), Some(90));
    std::fs::remove_dir_all(&d).ok();
}

#[test]
fn locals_generic_names_and_foreign_paths_are_not_edges() {
    let d = krate("fp");
    let g = build(&d);
    let run = node(&g, "run");
    let out: Vec<String> = g.semantic().filter(|e| e.0 == run).map(|e| g.nodes[e.1].name.clone()).collect();
    for bad in ["text", "f", "new"] { assert!(!out.contains(&bad.to_string()), "{bad} is a local, a closure or a foreign path: {out:?}"); }
    let e = g.explain("run");
    assert!(e.contains("calls (") && e.contains("uses_type (1): Dog") && e.contains("tested by (1)"), "{e}");
    std::fs::remove_dir_all(&d).ok();
}

#[test]
fn version_2_files_load_and_are_rebuilt() {
    let old = serde_json::json!({"v": 2, "files": 1, "nodes": [["a.py", "f", 1, 2, "function"], ["a.py", "T", 3, 4, "type"]], "edges": [[0, 1, 1]]});
    let g = Graph::from_json(&old).unwrap();
    assert_eq!((g.rel[0], g.score[0], g.conf[0]), (USES_TYPE, 70, INFERRED));
    let back = Graph::from_json(&g.to_json()).unwrap();
    assert_eq!((back.rel.clone(), back.score.clone()), (g.rel, g.score));
    let d = krate("v2");
    std::fs::create_dir_all(d.join(".vtok")).unwrap();
    std::fs::write(d.join(".vtok/graph.json"), old.to_string()).unwrap();
    let g = load_or_build(&d);
    assert!(g.nodes.len() > 2, "a stale-format file is rebuilt transparently");
    std::fs::remove_dir_all(&d).ok();
}

#[test]
fn graphing_src_finds_the_manifest_above_and_resolves_strictly() {
    let d = tmp("srcdir");
    put(&d, "Cargo.toml", "[package]\nname = \"b\"\n");
    put(&d, "src/lib.rs", "mod ext;\nmod other;\nuse ext::Ext;\n\npub enum Out { Full(u8), Part(u8) }\n\npub fn data() -> u8 { 1 }\n\npub fn show(o: Out, v: &mut Vec<u8>) -> u8 {\n    v.push_one(1);\n    match o { Out::Full(data) => data, Out::Part(data) => data + 1 }\n}\n");
    put(&d, "src/ext.rs", "pub trait Ext {\n    fn push_one(&mut self, x: u8);\n}\nimpl Ext for Vec<u8> {\n    fn push_one(&mut self, x: u8) { self.push(x) }\n}\n");
    put(&d, "src/other.rs", "pub fn helper() -> u8 { 2 }\npub fn caller() -> u8 { helper() }\n");
    put(&d, "src/lone.rs", "pub fn helper() -> u8 { 3 }\n");
    let g = build(&d.join("src"));
    let show = node(&g, "show");
    assert!(rel(&g, show, node(&g, "data"), REFERENCES).is_none(), "`data` is bound by the variant patterns");
    assert_eq!(rel(&g, show, node(&g, "Ext for Vec.push_one"), CALLS), Some(80), "one impl of an imported extension trait");
    let caller = node(&g, "caller");
    let out: Vec<&str> = g.semantic().filter(|e| e.0 == caller).map(|e| g.nodes[e.1].file.as_str()).collect();
    assert_eq!(out, vec!["other.rs"], "strict: the same-file helper, never the one in lone.rs");
    std::fs::remove_dir_all(&d).ok();
}
