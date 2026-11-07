//! The bulk-file guard: a file with thousands of symbols (generated bindings, constant tables) stays a file node.
use super::edges::{limit_from, BULK_SYMBOLS};
use super::tests::{put, tmp};
use super::tests_rels::{node, rel};
use super::*;

/// `n` one-line constants, the shape of windows-sys and libc: one symbol each.
fn table(n: usize) -> String { (0..n).map(|i| format!("pub const C{i}: u32 = {i};\n")).collect() }

#[test]
fn a_file_over_the_limit_stays_a_file_node_and_the_rest_is_graphed() {
    let d = tmp("bulk");
    put(&d, "src/real.rs", "pub fn leaf() -> u32 { 1 }\npub fn top() -> u32 { leaf() + 1 }\n");
    put(&d, "src/gen.rs", &format!("{}pub fn from_table() -> u32 {{ C1 }}\n", table(BULK_SYMBOLS + 100)));
    let g = build(&d);
    assert_eq!(g.bulk.len(), 1);
    assert!(g.bulk[0].0 == "src/gen.rs" && g.bulk[0].1 > BULK_SYMBOLS + 100, "{:?}", g.bulk);
    assert!(g.nodes.iter().any(|n| n.kind == "file" && n.name == "src/gen.rs"), "the file itself stays");
    assert!(!g.nodes.iter().any(|n| n.file == "src/gen.rs" && n.kind != "file"), "none of its symbols does");
    assert_eq!(rel(&g, node(&g, "top"), node(&g, "leaf"), CALLS), Some(100), "neighbours are graphed as before");
    let s = g.summary();
    assert!(s.contains("left out: 1 files of 1101+ symbols") && s.contains("the largest src/gen.rs with 1101"), "{s}");
    std::fs::remove_dir_all(&d).ok();
}

#[test]
fn the_limit_counts_symbols_and_a_file_at_it_is_kept() {
    assert_eq!(vtok_api::symbols("rust", &table(BULK_SYMBOLS)).unwrap().len(), BULK_SYMBOLS, "one symbol per const");
    let d = tmp("bulk-edge");
    put(&d, "at.rs", &table(BULK_SYMBOLS));
    put(&d, "over.rs", &table(BULK_SYMBOLS + 1));
    let g = build(&d);
    assert_eq!(g.bulk, vec![("over.rs".to_string(), BULK_SYMBOLS + 1)]);
    assert_eq!(g.nodes.iter().filter(|n| n.file == "at.rs" && n.kind == "value").count(), BULK_SYMBOLS);
    assert!(g.summary().contains("left out: 1 files of 1001+ symbols (1001 in all"), "only the file over the limit is counted: {}", g.summary());
    std::fs::remove_dir_all(&d).ok();
}

#[test]
fn a_graph_without_bulk_files_prints_no_note() {
    let d = tmp("bulk-none");
    put(&d, "a.rs", "pub fn f() -> u32 { 1 }\n");
    let g = build(&d);
    assert!(g.bulk.is_empty() && !g.summary().contains("left out"));
    std::fs::remove_dir_all(&d).ok();
}

#[test]
fn the_environment_value_sets_or_disables_the_limit() {
    assert_eq!(limit_from(None), BULK_SYMBOLS);
    assert_eq!(limit_from(Some("250")), 250);
    assert_eq!(limit_from(Some(" 0 ")), usize::MAX, "0 keeps every file");
    assert_eq!(limit_from(Some("many")), BULK_SYMBOLS, "a value that is not a number keeps the default");
}

#[test]
fn the_left_out_files_are_stored_with_the_graph_and_old_graphs_still_load() {
    let d = tmp("bulk-store");
    put(&d, "gen.rs", &table(BULK_SYMBOLS + 5));
    put(&d, "ok.rs", "pub fn f() -> u32 { 1 }\n");
    let g = build(&d);
    let mut v = g.to_json();
    assert_eq!(Graph::from_json(&v).unwrap().bulk, g.bulk);
    v.as_object_mut().unwrap().remove("bulk");
    assert!(Graph::from_json(&v).unwrap().bulk.is_empty(), "a graph stored before the guard has no such field");
    std::fs::remove_dir_all(&d).ok();
}
