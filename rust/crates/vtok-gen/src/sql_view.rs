//! SQL views, statement by statement (so they work on statements the grammar cannot parse). Outline: one name per
//! statement (`create table public.users`, `insert into t (120 rows)`...). Skeleton: statement heads only: tables with
//! column names and types (`pk`, `->ref` marks), function signatures with the body as `$$…N lines$$`, views without
//! their query, inserts as `…N rows`. Lean: the full view with inserts cut to their first 3 rows.
use crate::emit::Emit;
use crate::lex::{protected, regions};
use crate::spec::Spec;
use crate::sql_words::*;
use crate::stmts::statements;
use crate::tree::*;
use std::collections::BTreeMap;
use tree_sitter::Node;
use vtok_core::language::Unit;

fn short(s: String, n: usize) -> String { if s.chars().count() > n { s.chars().take(n).collect::<String>() + "…" } else { s } }

/// `name type` per column of a CREATE TABLE body, with ` pk` and ` ->table` marks; table constraints dropped.
fn columns(w: &[String]) -> String {
    let (mut cols, mut cur, mut depth) = (vec![], vec![], 0i32);
    for x in w {
        match x.as_str() { "(" => depth += 1, ")" => depth -= 1, _ => {} }
        if depth < 0 { break; }
        if depth == 0 && x == "," { cols.push(std::mem::take(&mut cur)); } else { cur.push(x.clone()); }
    }
    cols.push(cur);
    let mut out = vec![];
    for c in cols.iter().filter(|c| !c.is_empty()) {
        let l: Vec<String> = c.iter().map(|x| x.to_ascii_lowercase()).collect();
        if ["constraint", "primary", "foreign", "unique", "check", "exclude", "like", "index", "key"].contains(&l[0].as_str()) { continue; }
        let mut s = c[0].clone();
        if let Some(t) = c.get(1) { s += " "; s += t; }
        if c.get(2).map_or(false, |x| x == "(") { if let Some(e) = c.iter().position(|x| x == ")") { s += &c[2..=e].concat(); } }
        if l.windows(2).any(|p| p[0] == "primary" && p[1] == "key") { s += " pk"; }
        if let Some(r) = l.iter().position(|x| x == "references") { s += &format!(" ->{}", c.get(r + 1).cloned().unwrap_or_default()); }
        out.push(s);
    }
    out.join(",")
}

fn head(spec: &Spec, stmt: &str) -> String {
    let flat = flatten(spec, stmt, false);
    let w = words(&flat);
    let lw: Vec<String> = w.iter().map(|x| x.to_ascii_lowercase()).collect();
    let obj = lw.iter().skip(1).find(|x| !matches!(x.as_str(), "or" | "replace" | "unique" | "temp" | "temporary" | "materialized" | "unlogged" | "recursive")).cloned().unwrap_or_default();
    if lw.first().map(String::as_str) == Some("create") && obj == "table" {
        if let Some(p) = w.iter().position(|x| x == "(") { return format!("{}({})", w[..p].join(" "), columns(&w[p + 1..])); }
    }
    if lw.first().map(String::as_str) == Some("create") && obj == "view" {
        if let Some(a) = lw.iter().position(|x| x == "as") { return format!("{} as …", w[..a].join(" ")); }
    }
    if lw.first().map(String::as_str) == Some("insert") {
        if let Some(v) = lw.iter().position(|x| x == "values") { return format!("{} values …{} rows", flatten(spec, &w[..v].join(" "), false), rows(&w[v + 1..])); }
    }
    short(flat, if obj == "function" || obj == "procedure" { 400 } else { 160 })
}

fn texts(source: &str) -> Vec<(usize, usize, &str)> { statements(source).into_iter().map(|(a, b)| (a, b, &source[a..b])).collect() }

pub fn skeleton(spec: &Spec, source: &str) -> String {
    texts(source).into_iter().map(|(_, _, s)| head(spec, s)).filter(|h| !h.is_empty()).map(|h| h + "\n").collect()
}

/// Statements as outline / chunk units, named by `label`.
pub fn units(spec: &Spec, source: &str) -> Vec<Unit> {
    let line = |at: usize| source[..at.min(source.len())].matches('\n').count() + 1;
    let mut out = vec![];
    for (a, b, s) in texts(source) {
        let w = words(&flatten(spec, s, false));
        if w.is_empty() { continue; }
        // the statement starts at its first character outside comments and blanks
        let mut start = 0;
        for (x, e, string) in regions(spec, s) { if string || !s[start..x].trim().is_empty() { break; } start = e; }
        let first = a + start + (s[start..].len() - s[start..].trim_start().len());
        out.push(Unit { name: label(&w), kind: "statement".into(), start: line(first), end: line(b.saturating_sub(1).max(first)), member: false, parent: None, indent: 0 });
    }
    out
}

fn cut_rows(n: Node, em: &mut Emit) {
    if n.kind() == "insert" {
        let kids = children(n);
        let vals = kids.iter().position(|c| c.kind() == "keyword_values").unwrap_or(kids.len());
        let lists: Vec<usize> = (vals..kids.len()).filter(|&i| kids[i].kind() == "list").collect();
        if lists.len() > 3 {
            em.subst.insert(kids[lists[3]].id(), format!("/* +{} rows */", lists.len() - 3));
            for i in lists[2] + 1..kids.len() { if i != lists[3] && (kids[i].kind() == "list" || kids[i].kind() == ",") { em.skip.insert(kids[i].id()); } }
        }
    }
    for c in children(n) { cut_rows(c, em); }
}

pub fn lean(spec: &'static Spec, source: &str) -> String {
    let none = BTreeMap::new();
    let mut out = String::new();
    for (_, _, s) in texts(source) {
        let tree = parse(spec, s.as_bytes());
        let root = tree.root_node();
        let mut em = Emit::new(s.as_bytes(), spec, &none, false, false, &named_children(root));
        if root.has_error() { em.coarse = true; em.protect = protected(spec, s); }
        cut_rows(root, &mut em);
        let t = em.run_seg(&children(root)).0;
        if !t.trim().is_empty() { out.push_str(t.trim_end()); out.push('\n'); }
    }
    out
}
