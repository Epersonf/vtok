//! Structural relations: contains (file -> symbol, type -> its methods), implements / extends (`impl Trait for Type`,
//! `class A(B)`, `class A extends B implements C`, `trait A: B`, `interface A extends B`), imports (file -> file) and
//! tests (a test function -> the one function it exercises, when unambiguous).
use super::edges::{decl_line, put_rel, EdgeMap, Parsed};
use super::names::is_test;
use super::resolve::Ctx;
use super::scope::Scope;
use super::{Node, CALLS, CONTAINS, EXTENDS, IMPLEMENTS, IMPORTS, TESTS};
use std::collections::{HashMap, HashSet};

/// Names a class / trait / interface declaration line derives from: (extends, implements).
fn parents(lang: &str, decl: &str) -> (Vec<String>, Vec<String>) {
    let ids = |s: &str| -> Vec<String> { s.split(|c: char| !(c.is_alphanumeric() || c == '_' || c == '.')).filter(|w| w.starts_with(|c: char| c.is_alphabetic())).map(|w| w.rsplit('.').next().unwrap_or(w).to_string()).collect() };
    let d = decl.split('{').next().unwrap_or(decl);
    match lang {
        "python" => (d.split_once('(').map(|x| ids(x.1.split(')').next().unwrap_or("")).into_iter().filter(|w| w != "object" && !w.contains('=')).collect()).unwrap_or_default(), vec![]),
        "rust" => (d.split_once("trait ").and_then(|x| x.1.split_once(':')).map(|x| ids(x.1.split("where").next().unwrap_or(""))).unwrap_or_default(), vec![]),
        _ => {
            let ext = d.split_once(" extends ").map(|x| ids(x.1.split(" implements ").next().unwrap_or(""))).unwrap_or_default();
            let imp = d.split_once(" implements ").map(|x| ids(x.1)).unwrap_or_default();
            let colon = if lang == "csharp" || lang == "cpp" || lang == "dart" { d.split_once(':').map(|x| ids(x.1)).unwrap_or_default() } else { vec![] };
            ([ext, colon].concat(), imp)
        }
    }
}

fn type_node(cx: &Ctx, f: usize, name: &str) -> Option<usize> {
    let t = cx.named(f, name, &["type"]);
    cx.pick(f, name, &t).map(|x| x.0)
}

pub(crate) fn add(nodes: &[Node], parsed: &[Parsed], sc: &[Scope], by_base: &HashMap<String, Vec<usize>>, m: &mut EdgeMap) {
    let cx = Ctx::new(nodes, parsed, sc, by_base);
    let calls: HashMap<usize, Vec<usize>> = m.keys().filter(|k| k.2 == CALLS).fold(HashMap::new(), |mut acc, k| { acc.entry(k.0).or_default().push(k.1); acc });
    for (f, p) in parsed.iter().enumerate() {
        let lines: Vec<&str> = p.src.lines().collect();
        for (k, &(_, a, b)) in p.syms.iter().enumerate() {
            let Some(n) = p.node_of[k] else { continue };
            put_rel(m, p.file_node, n, CONTAINS, 100);
            let node = &nodes[n];
            if node.kind == "method" || (node.kind == "value" && !node.owner().is_empty()) {
                if let Some(t) = type_node(&cx, f, node.owner()) { put_rel(m, t, n, CONTAINS, 100); }
                if let (Some(tr), Some(t)) = (node.impl_trait(), type_node(&cx, f, node.owner())) {
                    if let Some(trn) = type_node(&cx, f, tr) { put_rel(m, t, trn, IMPLEMENTS, 100); }
                }
            }
            if node.kind == "type" {
                let (ext, imp) = parents(p.lang, decl_line(&lines, a, b));
                for (list, rel) in [(ext, EXTENDS), (imp, IMPLEMENTS)] {
                    for w in list { if let Some(t) = type_node(&cx, f, &w) { let r = if cx.parsed[f].traits.contains(&t) && rel == EXTENDS && p.lang != "rust" { IMPLEMENTS } else { rel }; put_rel(m, n, t, r, 100); } }
                }
            }
            let hi = (a + 2).min(lines.len());
            let attrs = lines[a.saturating_sub(4).min(hi)..hi].join(" ");
            if matches!(node.kind, "function" | "method") && is_test(&node.file, &node.name, &attrs) {
                if let Some((t, s)) = tested(nodes, n, calls.get(&n).map(|v| v.as_slice()).unwrap_or(&[])) { put_rel(m, n, t, TESTS, s); }
            }
        }
        let mut seen: HashSet<usize> = HashSet::new();
        let s = &sc[f];
        // `mod x;` children are contained, not imported, unless a `use` names them
        let used_mods = s.mods.iter().filter(|(k, v)| !(p.lang == "rust" && v.len() == 1 && s.children.get(*k) == Some(&v[0]))).flat_map(|x| x.1.iter());
        for g in s.names.values().flat_map(|x| x.1.iter()).chain(&s.globs).chain(used_mods) {
            if *g != f && seen.insert(*g) { put_rel(m, p.file_node, parsed[*g].file_node, IMPORTS, 100); }
        }
    }
}

/// The function a test exercises: the callee whose name the test's name contains (0.9), else its only callee (0.7).
fn tested(nodes: &[Node], t: usize, callees: &[usize]) -> Option<(usize, u8)> {
    let tn = crate::refs::base(&nodes[t].name).to_ascii_lowercase();
    let real: Vec<usize> = callees.iter().copied().filter(|&c| !is_test(&nodes[c].file, &nodes[c].name, "")).collect();
    let named: Vec<usize> = real.iter().copied().filter(|&c| { let b = crate::refs::base(&nodes[c].name).to_ascii_lowercase(); b.len() >= 3 && tn.contains(&b) }).collect();
    let longest = named.iter().map(|&c| crate::refs::base(&nodes[c].name).len()).max();
    let top: Vec<usize> = named.iter().copied().filter(|&c| Some(crate::refs::base(&nodes[c].name).len()) == longest).collect();
    if top.len() == 1 { return Some((top[0], 90)); }
    if real.len() == 1 { return Some((real[0], 70)); }
    None
}
