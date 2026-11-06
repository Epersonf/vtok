//! Python plugin: native-tight notation built from the tree-sitter CST.
//!
//! The text stays real Python (1-space indent, no optional whitespace, inline single-line bodies,
//! `;`-joined simple statements, comments and docstrings removed, aliased names). Decoding only
//! reverses aliases and hidden literals, so the model's output is valid Python as written.
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, HashSet};
use tree_sitter::{Node, Parser, Tree};
use vtok_core::aliasing::plan_aliases;
use vtok_core::tokens::ntok;
use vtok_core::{Result, VtokError};
pub use vtok_core::lang::{EncodeOpts, Encoded, View};

static PY_NAMES: &str = include_str!("../../vtok-core/data_py_names.txt");
const IMPORTS: [&str; 3] = ["import_statement", "import_from_statement", "future_import_statement"];
const CLAUSES: [&str; 6] = ["elif_clause", "else_clause", "except_clause", "except_group_clause", "finally_clause", "case_clause"];

pub fn parse(src: &[u8]) -> Tree {
    let mut p = Parser::new();
    p.set_language(&tree_sitter_python::LANGUAGE.into()).expect("python grammar");
    p.parse(src, None).expect("parse")
}

pub fn first_error<'a>(n: Node<'a>) -> Option<Node<'a>> {
    if n.kind() == "ERROR" || n.is_missing() { return Some(n); }
    if !n.has_error() { return None; }
    for ch in children(n) {
        if let Some(e) = first_error(ch) { return Some(e); }
    }
    Some(n)
}

fn children<'a>(n: Node<'a>) -> Vec<Node<'a>> { let mut c = n.walk(); n.children(&mut c).collect() }
fn named_children<'a>(n: Node<'a>) -> Vec<Node<'a>> { let mut c = n.walk(); n.named_children(&mut c).collect() }
fn text<'a>(n: Node, src: &'a [u8]) -> &'a str { std::str::from_utf8(&src[n.start_byte()..n.end_byte()]).unwrap_or("") }
fn trivia(n: Node) -> bool { matches!(n.kind(), "comment" | "line_continuation") }

// ---------------------------------------------------------------- strings / docstrings

fn has_interpolation(n: Node) -> bool { children(n).iter().any(|c| c.kind() == "interpolation") }

/// `string` node that is a plain (non-f, non-bytes) str constant.
fn is_plain_str(n: Node, src: &[u8]) -> bool {
    if n.kind() != "string" || has_interpolation(n) { return false; }
    let start = children(n).into_iter().find(|c| c.kind() == "string_start");
    let pre = start.map(|s| text(s, src).to_lowercase()).unwrap_or_default();
    let prefix: String = pre.chars().take_while(|c| c.is_alphabetic()).collect();
    !prefix.contains('b') && !prefix.contains('f') && !prefix.contains('t')
}

fn is_docstring(stmt: Node, src: &[u8]) -> bool {
    if stmt.kind() != "expression_statement" { return false; }
    let kids: Vec<Node> = named_children(stmt).into_iter().filter(|c| !trivia(*c)).collect();
    if kids.len() != 1 { return false; }
    let k = kids[0];
    match k.kind() {
        "string" => is_plain_str(k, src),
        "concatenated_string" => named_children(k).iter().all(|s| s.kind() == "string" && is_plain_str(*s, src)),
        _ => false,
    }
}

fn body_statements<'a>(block: Node<'a>, strip_doc: bool, src: &[u8]) -> Vec<Node<'a>> {
    let mut v: Vec<Node> = named_children(block).into_iter().filter(|c| !trivia(*c)).collect();
    if strip_doc && v.first().map_or(false, |f| is_docstring(*f, src)) { v.remove(0); }
    v
}

// ---------------------------------------------------------------- emission

struct Ctx<'a> {
    ren: &'a BTreeMap<String, String>,
    lean: bool,
    skeleton: bool,
    hide: usize,
    existing: &'a HashSet<String>,
    lits: RefCell<(BTreeMap<String, String>, HashMap<String, String>, usize)>, // placeholder->text, text->placeholder, counter
    src: &'a [u8],
}

struct Line { indent: usize, text: String, simple: bool }

fn isword_end(s: &str) -> bool { s.chars().last().map_or(false, |c| c.is_alphanumeric() || c == '_' || c == '\'' || c == '"') }
fn isword_start(s: &str) -> bool { s.chars().next().map_or(false, |c| c.is_alphanumeric() || c == '_' || c == '\'' || c == '"') }

fn join(tokens: &[String]) -> String {
    let mut out = String::new();
    for (i, t) in tokens.iter().enumerate() {
        if i > 0 {
            let p = &tokens[i - 1];
            let numdot = !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()) && t == ".";
            if (isword_end(p) && isword_start(t)) || numdot { out.push(' '); }
        }
        out.push_str(t);
    }
    out
}

fn rename_of(s: &str, ctx: &Ctx) -> String { ctx.ren.get(s).cloned().unwrap_or_else(|| s.to_string()) }

/// Source text of `n` with identifier descendants renamed (used for f-strings).
fn text_renamed(n: Node, ctx: &Ctx) -> String {
    let mut spans: Vec<(usize, usize, String)> = vec![];
    let mut st = vec![n];
    while let Some(x) = st.pop() {
        for c in children(x) { st.push(c); }
        if x.kind() == "identifier" {
            let s = text(x, ctx.src);
            if let Some(r) = ctx.ren.get(s) { spans.push((x.start_byte() - n.start_byte(), x.end_byte() - n.start_byte(), r.clone())); }
        }
    }
    spans.sort_by(|a, b| b.0.cmp(&a.0));
    let mut b = text(n, ctx.src).as_bytes().to_vec();
    for (a, e, r) in spans { b.splice(a..e, r.into_bytes()); }
    String::from_utf8_lossy(&b).into_owned()
}

fn literal_placeholder(s: &str, ctx: &Ctx) -> String {
    let mut g = ctx.lits.borrow_mut();
    if let Some(p) = g.1.get(s) { return p.clone(); }
    let mut k = g.2;
    loop {
        k += 1;
        let ph = format!("'${k}'");
        if !ctx.existing.contains(&ph) { g.2 = k; g.0.insert(ph.clone(), s.to_string()); g.1.insert(s.to_string(), ph.clone()); return ph; }
    }
}

fn is_field(parent: Node, child: Node, name: &str) -> bool {
    parent.child_by_field_name(name).map_or(false, |f| f.id() == child.id())
}

fn flatten(n: Node, ctx: &Ctx, in_import: bool) -> Vec<String> {
    let t = n.kind();
    if trivia(n) { return vec![]; }
    match t {
        "identifier" => {
            let s = text(n, ctx.src);
            return vec![if in_import { s.to_string() } else { rename_of(s, ctx) }];
        }
        "string" => {
            if has_interpolation(n) { return vec![text_renamed(n, ctx)]; }
            let s = text(n, ctx.src);
            if ctx.hide > 0 && is_plain_str(n, ctx.src) && ntok(s) >= ctx.hide { return vec![literal_placeholder(s, ctx)]; }
            return vec![s.to_string()];
        }
        _ => {}
    }
    let imp = in_import || IMPORTS.contains(&t);
    if n.child_count() == 0 { return vec![text(n, ctx.src).to_string()]; }
    let mut out: Vec<String> = vec![];
    for c in children(n) {
        if ctx.lean && (is_field(n, c, "type") || is_field(n, c, "return_type")) {
            if matches!(out.last().map(|s| s.as_str()), Some(":") | Some("->")) { out.pop(); }
            continue;
        }
        out.extend(flatten(c, ctx, imp));
    }
    out
}

fn simple_line(n: Node, ctx: &Ctx, indent: usize) -> Line {
    Line { indent, text: join(&flatten(n, ctx, false)), simple: true }
}

fn emit_stmt(n: Node, ctx: &Ctx, indent: usize, out: &mut Vec<Line>) {
    let t = n.kind();
    if trivia(n) { return; }
    if t == "decorated_definition" {
        for c in children(n) {
            if trivia(c) { continue; }
            if c.kind() == "decorator" {
                out.push(Line { indent, text: join(&flatten(c, ctx, false)), simple: false });
            } else {
                emit_stmt(c, ctx, indent, out);
            }
        }
        return;
    }
    let compound = matches!(t, "if_statement" | "for_statement" | "while_statement" | "try_statement" | "with_statement"
        | "function_definition" | "class_definition" | "match_statement") || CLAUSES.contains(&t);
    if !compound { out.push(simple_line(n, ctx, indent)); return; }
    let is_def = matches!(t, "function_definition" | "class_definition");
    let mut header: Vec<String> = vec![];
    for c in children(n) {
        if trivia(c) { continue; }
        let k = c.kind();
        if k == "block" {
            let mut body: Vec<Line> = vec![];
            if ctx.skeleton && t == "function_definition" {
                body.push(Line { indent: indent + 1, text: "...".into(), simple: true });
            } else {
                for s in body_statements(c, is_def, ctx.src) { emit_stmt(s, ctx, indent + 1, &mut body); }
                if body.is_empty() { body.push(Line { indent: indent + 1, text: "pass".into(), simple: true }); }
            }
            let head = join(&header);
            header.clear();
            if !is_def && body.iter().all(|l| l.simple) {
                let joined = body.iter().map(|l| l.text.as_str()).collect::<Vec<_>>().join(";");
                out.push(Line { indent, text: head + &joined, simple: false });
            } else {
                out.push(Line { indent, text: head, simple: false });
                out.extend(body);
            }
        } else if CLAUSES.contains(&k) {
            if !header.is_empty() { out.push(Line { indent, text: join(&header), simple: false }); header.clear(); }
            emit_stmt(c, ctx, indent, out);
        } else if ctx.lean && is_field(n, c, "return_type") {
            if header.last().map(|s| s.as_str()) == Some("->") { header.pop(); }
        } else {
            header.extend(flatten(c, ctx, false));
        }
    }
    if !header.is_empty() { out.push(Line { indent, text: join(&header), simple: false }); }
}

fn render(lines: Vec<Line>) -> String {
    let mut res: Vec<Line> = vec![];
    for l in lines {
        if let Some(last) = res.last_mut() {
            if last.indent == l.indent && last.simple && l.simple {
                last.text.push(';');
                last.text.push_str(&l.text);
                continue;
            }
        }
        res.push(l);
    }
    let mut s = String::new();
    for l in res { s.push_str(&" ".repeat(l.indent)); s.push_str(&l.text); s.push('\n'); }
    s
}

// ---------------------------------------------------------------- analysis

pub struct Analysis { pub counts: HashMap<String, usize>, pub excluded: HashSet<String>, pub avoid: HashSet<String> }

fn first_ident(n: Node, src: &[u8]) -> Option<String> {
    if n.kind() == "identifier" { return Some(text(n, src).to_string()); }
    for c in children(n) { if let Some(s) = first_ident(c, src) { return Some(s); } }
    None
}

pub fn analyze(root: Node, src: &[u8]) -> Analysis {
    let mut counts: HashMap<String, usize> = HashMap::new();
    let mut excluded: HashSet<String> = HashSet::new();
    let mut avoid: HashSet<String> = PY_NAMES.lines().map(String::from).collect();
    let mut st = vec![(root, false)];
    while let Some((n, in_imp)) = st.pop() {
        let k = n.kind();
        let imp = in_imp || IMPORTS.contains(&k);
        if k == "identifier" {
            let s = text(n, src).to_string();
            avoid.insert(s.clone());
            if !imp { *counts.entry(s).or_insert(0) += 1; }
        }
        if k == "import_statement" || k == "import_from_statement" {
            let module = n.child_by_field_name("module_name").map(|m| m.id());
            for c in named_children(n) {
                if Some(c.id()) == module { continue; }
                match c.kind() {
                    "dotted_name" => { if let Some(s) = first_ident(c, src) { excluded.insert(s); } }
                    "aliased_import" => {
                        if let Some(a) = c.child_by_field_name("alias") { excluded.insert(text(a, src).to_string()); }
                    }
                    _ => {}
                }
            }
        }
        for c in children(n) { st.push((c, imp)); }
    }
    Analysis { counts, excluded, avoid }
}

// ---------------------------------------------------------------- canonical dump (verification)

fn canon(n: Node, src: &[u8], strip_docs: bool, out: &mut Vec<String>) {
    let t = n.kind();
    if trivia(n) { return; }
    if !n.is_named() {
        if t != ";" { out.push(t.to_string()); }
        return;
    }
    if t == "string" { out.push(format!("string:{}", text(n, src))); return; }
    if n.child_count() == 0 && t != "module" { out.push(format!("{t}:{}", text(n, src))); return; }
    out.push(format!("({t}"));
    let body_owner = strip_docs && matches!(t, "module" | "block") && n.parent().map_or(true, |p| matches!(p.kind(), "function_definition" | "class_definition") || t == "module");
    let mut first_stmt = body_owner;
    let mut emitted = false;
    for c in children(n) {
        if trivia(c) { continue; }
        if first_stmt && c.is_named() {
            first_stmt = false;
            if is_docstring(c, src) { continue; }
        }
        if c.is_named() || c.kind() != ";" { emitted |= c.is_named(); }
        canon(c, src, strip_docs, out);
    }
    if body_owner && t == "block" && !emitted {
        out.extend(["(pass_statement".to_string(), "pass".to_string(), ")".to_string()]);
    }
    out.push(")".into());
}

// ---------------------------------------------------------------- public API

pub fn encode(source: &str, opts: &EncodeOpts) -> Result<Encoded> {
    let source = source.replace("\r\n", "\n");
    let src = source.as_bytes();
    let tree = parse(src);
    let root = tree.root_node();
    if let Some(e) = first_error(root) {
        return Err(VtokError::at("source has syntax errors", e.start_position().row + 1));
    }
    let an = analyze(root, src);
    let table: BTreeMap<String, String> = if let Some(t) = &opts.table {
        t.clone()
    } else if opts.alias {
        plan_aliases(&an.counts, &an.avoid, &HashSet::new(), &an.excluded, 2)
    } else {
        BTreeMap::new()
    };
    let mut existing: HashSet<String> = HashSet::new();
    if opts.hide_literals > 0 {
        let mut st = vec![root];
        while let Some(n) = st.pop() {
            if n.kind() == "string" { existing.insert(text(n, src).to_string()); }
            for c in children(n) { st.push(c); }
        }
    }
    let ctx = Ctx {
        ren: &table, lean: opts.view == View::Lean, skeleton: opts.view == View::Skeleton, hide: opts.hide_literals,
        existing: &existing, lits: RefCell::new((BTreeMap::new(), HashMap::new(), 0)), src,
    };
    let mut lines: Vec<Line> = vec![];
    for s in body_statements(root, true, src) { emit_stmt(s, &ctx, 0, &mut lines); }
    let vt = render(lines);
    let lits = ctx.lits.borrow().0.clone();
    if opts.verify && opts.view == View::Full {
        let inv: HashMap<String, String> = table.iter().map(|(o, a)| (a.clone(), o.clone())).collect();
        let back = decode(&vt, &inv, &lits.iter().map(|(k, v)| (k.clone(), v.clone())).collect())?;
        let bt = parse(back.as_bytes());
        let (mut a, mut b) = (vec![], vec![]);
        canon(bt.root_node(), back.as_bytes(), false, &mut a);
        canon(root, src, true, &mut b);
        if a != b { return Err(VtokError::new("roundtrip verification failed (tree differs)")); }
    } else if opts.verify {
        if let Some(e) = first_error(parse(vt.as_bytes()).root_node()) {
            return Err(VtokError::at("generated notation does not parse", e.start_position().row + 1));
        }
    }
    Ok(Encoded { vtok: vt, table, literals: lits })
}

/// table: {alias: original}; literals: {placeholder: original source text}.
pub fn decode_raw(vtok: &str, table: &HashMap<String, String>, literals: &HashMap<String, String>) -> Result<String> {
    let mut code = vtok.to_string();
    if !code.ends_with('\n') { code.push('\n'); }
    let tree = parse(code.as_bytes());
    let root = tree.root_node();
    if let Some(e) = first_error(root) {
        let lines: Vec<&str> = code.lines().collect();
        let row = e.start_position().row.min(lines.len().saturating_sub(1));
        let near: String = lines.get(row).map(|l| l.trim().chars().take(60).collect()).unwrap_or_default();
        return Err(VtokError::at(format!("syntax error near {near:?}"), row + 1));
    }
    if table.is_empty() && literals.is_empty() { return Ok(code); }
    let raw = code.as_bytes();
    let mut spans: Vec<(usize, usize, String)> = vec![];
    let mut st = vec![(root, false)];
    while let Some((n, in_imp)) = st.pop() {
        let k = n.kind();
        let imp = in_imp || IMPORTS.contains(&k);
        let s = text(n, raw);
        if k == "string" && literals.contains_key(s) {
            spans.push((n.start_byte(), n.end_byte(), literals[s].clone()));
            continue;
        }
        if k == "identifier" && !imp {
            if let Some(r) = table.get(s) { spans.push((n.start_byte(), n.end_byte(), r.clone())); }
        }
        for c in children(n) { st.push((c, imp)); }
    }
    spans.sort_by(|a, b| b.0.cmp(&a.0));
    let mut out = raw.to_vec();
    for (a, b, r) in spans { out.splice(a..b, r.into_bytes()); }
    Ok(String::from_utf8_lossy(&out).into_owned())
}

/// Reverse aliases and literals, then lay the statements out with ordinary 4-space indentation
/// (expressions stay compact). Valid Python either way.
pub fn decode(vtok: &str, table: &HashMap<String, String>, literals: &HashMap<String, String>) -> Result<String> {
    let raw = decode_raw(vtok, table, literals)?;
    Ok(pretty(&raw, &Docs::default(), 0))
}

// ---------------------------------------------------------------- pretty printer + docstring reattachment

#[derive(Default, Clone)]
pub struct Docs {
    pub module: Option<(String, bool)>,
    pub map: HashMap<(String, usize), (String, bool)>,
}

fn name_of(n: Node, src: &[u8]) -> String {
    n.child_by_field_name("name").map(|x| text(x, src).to_string()).unwrap_or_default()
}

fn def_of<'a>(n: Node<'a>) -> Option<Node<'a>> {
    match n.kind() {
        "function_definition" | "class_definition" => Some(n),
        "decorated_definition" => n.child_by_field_name("definition"),
        _ => None,
    }
}

fn collect_walk(node: Node, src: &[u8], prefix: &str, seen: &mut HashMap<String, usize>, out: &mut HashMap<(String, usize), (String, bool)>) {
    for n in named_children(node) {
        if trivia(n) { continue; }
        if let Some(d) = def_of(n) {
            let q = format!("{prefix}{}", name_of(d, src));
            let k = { let e = seen.entry(q.clone()).and_modify(|x| *x += 1).or_insert(0); *e };
            if let Some(b) = d.child_by_field_name("body") {
                let stmts = body_statements(b, false, src);
                if stmts.first().map_or(false, |f| is_docstring(*f, src)) {
                    out.insert((q.clone(), k), (text(stmts[0], src).to_string(), stmts.len() == 1));
                }
                collect_walk(b, src, &format!("{q}."), seen, out);
            }
            // decorators / parameters may hold lambdas but never defs: nothing else to walk
        } else {
            collect_walk(n, src, prefix, seen, out);
        }
    }
}

/// Docstrings of `stmts` (a module body, or the members of a class) keyed by qualified name.
pub fn collect_docs(stmts: &[Node], src: &[u8]) -> Docs {
    let mut d = Docs::default();
    if stmts.first().map_or(false, |f| is_docstring(*f, src)) {
        d.module = Some((text(stmts[0], src).to_string(), stmts.len() == 1));
    }
    let mut seen = HashMap::new();
    for n in stmts {
        if let Some(df) = def_of(*n) {
            let q = name_of(df, src);
            let k = { let e = seen.entry(q.clone()).and_modify(|x| *x += 1).or_insert(0); *e };
            if let Some(b) = df.child_by_field_name("body") {
                let st = body_statements(b, false, src);
                if st.first().map_or(false, |f| is_docstring(*f, src)) {
                    d.map.insert((q.clone(), k), (text(st[0], src).to_string(), st.len() == 1));
                }
                collect_walk(b, src, &format!("{q}."), &mut seen, &mut d.map);
            }
        } else {
            collect_walk(*n, src, "", &mut seen, &mut d.map);
        }
    }
    d
}

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

mod plugin;
pub use plugin::Python;
