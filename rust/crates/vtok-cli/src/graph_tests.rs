//! Tests for symbol edges, confidence tags and graph storage.
use super::*;
use std::path::PathBuf;

pub(crate) fn tmp(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("vtok-graph-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

pub(crate) fn put(d: &Path, rel: &str, body: &str) {
    let p = d.join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, body).unwrap();
}

fn fixture(tag: &str) -> PathBuf {
    let d = tmp(tag);
    put(&d, "pkg/a.py", "def leaf(x):\n    return x\n\ndef mid(x):\n    return leaf(x)\n");
    put(&d, "pkg/b.py", "from a import mid\n\ndef top():\n    return mid(1)\n\nclass K:\n    def m(self):\n        return top()\n\ndef lone():\n    return 1\n");
    put(&d, "NOTES.md", "See `top()` and `K.m`.\n");
    d
}

fn id(g: &Graph, n: &str, file: &str) -> usize { g.nodes.iter().position(|x| x.name == n && x.file.contains(file)).unwrap_or_else(|| panic!("{n} in {file}")) }
fn conf(g: &Graph, a: usize, b: usize) -> Option<u8> { g.edges.iter().position(|&e| e == (a, b)).map(|i| g.conf[i]) }

#[test]
fn edges_follow_calls_and_docs_link_to_symbols() {
    let d = fixture("a");
    let g = build(&d);
    let id = |n: &str| g.nodes.iter().position(|x| x.name == n).unwrap();
    assert!(g.edges.contains(&(id("mid"), id("leaf"))) && g.edges.contains(&(id("top"), id("mid"))) && g.edges.contains(&(id("K.m"), id("top"))));
    assert!(!g.semantic().any(|e| e.0 == id("lone") || e.1 == id("lone")));
    assert!(g.nodes.iter().any(|n| n.kind == "doc" && n.name == "NOTES.md"));
    let p = g.path("K.m", "leaf");
    assert!(p.contains("3 hops") && p.contains("K.m") && p.contains("leaf") && p.contains("confidence EXTRACTED"), "{p}");
    let e = g.explain("mid");
    assert!(e.contains("called by (1): top") && e.contains("calls (1): leaf pkg/a.py:1\n"), "{e}");
    assert!(g.hubs(3).contains("mid"));
    std::fs::remove_dir_all(&d).ok();
}

#[test]
fn homonyms_are_narrowed_by_imports_directory_and_receiver_or_dropped() {
    let d = tmp("homonyms");
    put(&d, "a/util.py", "def helper():\n    return 1\n");
    put(&d, "b/other.py", "def helper():\n    return 2\n");
    put(&d, "a/main.py", "from util import helper\n\ndef run():\n    return helper()\n");
    put(&d, "c/lonely.py", "def stray():\n    return helper()\n");
    put(&d, "d/models.py", "class Repo:\n    def save(self):\n        return 1\n\nclass Cache:\n    def save(self):\n        return 2\n");
    put(&d, "e/use.py", "def go(repo, thing):\n    repo.save()\n    thing.save()\n\ndef only():\n    return 1\n");
    put(&d, "e/solo.py", "def unique_fn():\n    return 1\n\ndef caller():\n    return unique_fn()\n");
    let g = build(&d);
    let (run, stray, go) = (id(&g, "run", "main.py"), id(&g, "stray", "lonely"), id(&g, "go", "use.py"));
    assert_eq!(conf(&g, run, id(&g, "helper", "a/util")), Some(INFERRED));
    assert_eq!(conf(&g, run, id(&g, "helper", "b/other")), None);
    assert!(!g.edges.iter().any(|e| e.0 == stray), "unnarrowable homonyms get no edge");
    assert_eq!(conf(&g, go, id(&g, "Repo.save", "models")), Some(INFERRED));
    assert_eq!(conf(&g, go, id(&g, "Cache.save", "models")), None, "receiver `repo` picks Repo, `thing` matches nothing");
    assert_eq!(conf(&g, id(&g, "caller", "solo"), id(&g, "unique_fn", "solo")), Some(EXTRACTED));
    let e = g.explain("run");
    assert!(e.contains("calls (1): helper a/util.py:1 0.90"), "{e}");
    assert!(g.path("run", "helper").contains("confidence INFERRED"));
    let back = Graph::from_json(&g.to_json()).unwrap();
    assert_eq!(back.conf, g.conf);
    std::fs::remove_dir_all(&d).ok();
}

#[test]
fn imports_resolve_homonyms_in_typescript_go_and_rust() {
    let d = tmp("langs");
    put(&d, "src/a.ts", "export function fmt(x: number) { return x; }\n");
    put(&d, "lib/b.ts", "export function fmt(x: number) { return x + 1; }\n");
    put(&d, "app/c.ts", "import { fmt } from '../lib/b';\nexport function show() { return fmt(1); }\n");
    put(&d, "net/send.go", "package net\n\nfunc Send() int { return 1 }\n");
    put(&d, "mail/send.go", "package mail\n\nfunc Send() int { return 2 }\n");
    put(&d, "cmd/main.go", "package main\n\nimport \"x/net\"\n\nfunc start() int { return net.Send() }\n");
    put(&d, "r/foo.rs", "pub struct Foo;\nimpl Foo {\n    pub fn make() -> Foo { Foo }\n}\n");
    put(&d, "r/bar.rs", "pub struct Bar;\nimpl Bar {\n    pub fn make() -> Bar { Bar }\n}\n");
    put(&d, "r/use_it.rs", "fn build() {\n    let _f = Foo::make();\n}\n");
    let g = build(&d);
    assert_eq!(conf(&g, id(&g, "show", "app/c.ts"), id(&g, "fmt", "lib/b.ts")), Some(INFERRED));
    assert_eq!(conf(&g, id(&g, "show", "app/c.ts"), id(&g, "fmt", "src/a.ts")), None);
    assert_eq!(conf(&g, id(&g, "start", "main.go"), id(&g, "Send", "net/")), Some(INFERRED));
    assert_eq!(conf(&g, id(&g, "start", "main.go"), id(&g, "Send", "mail/")), None);
    let b = id(&g, "build", "use_it.rs");
    let to_make: Vec<usize> = g.edges.iter().filter(|e| e.0 == b).map(|e| e.1).collect();
    assert!(to_make.iter().all(|&t| g.nodes[t].file.ends_with("foo.rs")) && !to_make.iter().any(|&t| g.nodes[t].file.ends_with("bar.rs")), "{:?}", to_make.iter().map(|&t| g.short(t)).collect::<Vec<_>>());
    std::fs::remove_dir_all(&d).ok();
}

#[test]
fn stored_graph_round_trips_and_goes_stale() {
    let d = fixture("b");
    let a = load_or_build(&d);
    assert!(d.join(".vtok/graph.json").exists());
    let b = load_or_build(&d);
    assert_eq!((a.nodes.len(), a.edges.len()), (b.nodes.len(), b.edges.len()));
    // version 1 files (edges without a confidence) still load
    let old = serde_json::json!({"v": 1, "files": 1, "nodes": [["a.py", "f", 1, 2, "function"], ["a.py", "g", 3, 4, "function"]], "edges": [[0, 1]]});
    let o = Graph::from_json(&old).unwrap();
    assert_eq!(o.conf, vec![EXTRACTED]);
    std::thread::sleep(std::time::Duration::from_millis(1100));
    std::fs::write(d.join("pkg/c.py"), "def extra():\n    return 2\n").unwrap();
    assert_eq!(load_or_build(&d).nodes.len(), a.nodes.len() + 2, "a newer source file (a symbol and its file node) triggers a rebuild");
    std::fs::remove_dir_all(&d).ok();
}
