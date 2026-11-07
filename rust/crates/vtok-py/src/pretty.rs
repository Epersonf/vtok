//! Pretty printer: statement-level layout with ordinary 4-space indentation, reattaching docstrings.
use crate::ctx::{join, Ctx, Line};
use crate::docs::{name_of, Docs};
use crate::emit::{flatten, simple_line};
use crate::tree::{children, is_docstring, named_children, parse, trivia};
use crate::CLAUSES;
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, HashSet};
use tree_sitter::Node;

struct PState<'a> { docs: &'a Docs, seen: HashMap<String, usize> }

fn is_pass(n: Node) -> bool {
    n.kind() == "pass_statement"
}

fn pretty_body(stmts: &[Node], ctx: &Ctx, level: usize, prefix: &str, st: &mut PState, doc: Option<&(String, bool)>, out: &mut Vec<Line>) {
    let first_is_doc = stmts.first().map_or(false, |f| is_docstring(*f, ctx.src));
    let mut start = 0;
    if let Some((d, only)) = doc {
        if !first_is_doc {
            out.push(Line { indent: level, text: d.clone(), simple: true });
            if *only && stmts.len() == 1 && is_pass(stmts[0]) { start = 1; }
        }
    }
    for n in &stmts[start..] { pretty_stmt(*n, ctx, level, prefix, st, out); }
}

fn pretty_stmt(n: Node, ctx: &Ctx, level: usize, prefix: &str, st: &mut PState, out: &mut Vec<Line>) {
    let t = n.kind();
    if trivia(n) { return; }
    if t == "decorated_definition" {
        for c in children(n) {
            if trivia(c) { continue; }
            if c.kind() == "decorator" { out.push(Line { indent: level, text: join(&flatten(c, ctx, false)), simple: false }); }
            else { pretty_stmt(c, ctx, level, prefix, st, out); }
        }
        return;
    }
    let compound = matches!(t, "if_statement" | "for_statement" | "while_statement" | "try_statement" | "with_statement"
        | "function_definition" | "class_definition" | "match_statement") || CLAUSES.contains(&t);
    if !compound { out.push(simple_line(n, ctx, level)); return; }
    let is_def = matches!(t, "function_definition" | "class_definition");
    let mut header: Vec<String> = vec![];
    for c in children(n) {
        if trivia(c) { continue; }
        let k = c.kind();
        if k == "block" {
            out.push(Line { indent: level, text: join(&header), simple: false });
            header.clear();
            let stmts: Vec<Node> = named_children(c).into_iter().filter(|x| !trivia(*x)).collect();
            if is_def {
                let q = format!("{prefix}{}", name_of(n, ctx.src));
                let kk = { let e = st.seen.entry(q.clone()).and_modify(|x| *x += 1).or_insert(0); *e };
                let doc = st.docs.map.get(&(q.clone(), kk)).cloned();
                pretty_body(&stmts, ctx, level + 1, &format!("{q}."), st, doc.as_ref(), out);
            } else {
                pretty_body(&stmts, ctx, level + 1, prefix, st, None, out);
            }
        } else if CLAUSES.contains(&k) {
            if !header.is_empty() { out.push(Line { indent: level, text: join(&header), simple: false }); header.clear(); }
            pretty_stmt(c, ctx, level, prefix, st, out);
        } else {
            header.extend(flatten(c, ctx, false));
        }
    }
    if !header.is_empty() { out.push(Line { indent: level, text: join(&header), simple: false }); }
}

/// Statement-level layout of `code` at column `base` (spaces), reattaching `docs`.
pub fn pretty(code: &str, docs: &Docs, base: usize) -> String {
    let tree = parse(code.as_bytes());
    let root = tree.root_node();
    let src = code.as_bytes();
    let none = BTreeMap::new();
    let empty: HashSet<String> = HashSet::new();
    let ctx = Ctx { ren: &none, lean: false, skeleton: false, hide: 0, existing: &empty, lits: RefCell::new((BTreeMap::new(), HashMap::new(), 0)), src };
    let stmts: Vec<Node> = named_children(root).into_iter().filter(|x| !trivia(*x)).collect();
    let mut st = PState { docs, seen: HashMap::new() };
    let mut lines: Vec<Line> = vec![];
    pretty_body(&stmts, &ctx, 0, "", &mut st, docs.module.as_ref(), &mut lines);
    let mut s = String::new();
    for l in lines {
        s.push_str(&" ".repeat(base + 4 * l.indent));
        s.push_str(&l.text);
        s.push('\n');
    }
    s
}
