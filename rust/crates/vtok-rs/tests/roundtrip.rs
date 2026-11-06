use vtok_core::language::Language;
use vtok_rs::{EncodeOpts, Rust, View};

const SRC: &str = r#"
//! docs
use std::collections::HashMap;

/// A counter.
#[derive(Debug, Clone)]
pub struct Counter<'a> { name: &'a str, total_count: usize }

impl<'a> Counter<'a> {
    pub fn bump(&mut self, amount_to_add: usize) -> usize {
        // comment
        self.total_count += amount_to_add;
        let total_count = self.total_count;
        println!("{total_count} {}", self.name);
        total_count
    }
}

fn nested(v: Vec<Vec<u8>>) -> bool { v.len() >= 2 && v[0].len() != 0 || (1..3).contains(&2) }
"#;

fn rt(opts: EncodeOpts) {
    let e = Rust.encode(SRC, &opts).unwrap();
    let inv = e.table.iter().map(|(o, a)| (a.clone(), o.clone())).collect();
    let lits = e.literals.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
    let back = Rust.decode(&e.vtok, &inv, &lits).unwrap();
    assert!(Rust.equivalent(SRC, &back, false), "{}", e.vtok);
}

#[test]
fn full_roundtrip() { rt(EncodeOpts::default()); }

#[test]
fn hidden_literals_roundtrip() { rt(EncodeOpts { hide_literals: 1, ..Default::default() }); }

#[test]
fn views_parse() {
    for v in [View::Skeleton, View::Lean] {
        let e = Rust.encode(SRC, &EncodeOpts { view: v, ..Default::default() }).unwrap();
        assert!(!e.vtok.contains("comment"));
    }
}

#[test]
fn format_captures_are_not_aliased() {
    let e = Rust.encode(SRC, &EncodeOpts::default()).unwrap();
    assert!(!e.table.contains_key("total_count"));
}
