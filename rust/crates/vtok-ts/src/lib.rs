//! TypeScript plugin: native minified notation (port of the Python reference implementation).
use std::collections::{BTreeMap, HashMap, HashSet};
use tree_sitter::{Node, Parser, Tree};
use vtok_core::aliasing::plan_aliases;
use vtok_core::tokens::ntok;
use vtok_core::{Result, VtokError};

pub const IDENT: [&str; 3] = ["identifier", "type_identifier", "property_identifier"];
const LIKE_IDENT: [&str; 7] = [
    "identifier", "type_identifier", "property_identifier", "shorthand_property_identifier",
    "shorthand_property_identifier_pattern", "statement_identifier", "private_property_identifier",
];
const SKIP: [&str; 2] = ["comment", "empty_statement"];
const JS_WORDS: &str = "abstract any as async await boolean break case catch class const continue debugger declare default delete do else enum export extends false finally for from function get if implements import in instanceof interface is keyof let module namespace never new null number object of override package private protected public readonly return satisfies set static string super switch symbol this throw true try type typeof undefined unknown var void while with yield";
const KW_VALUES: &str = "f C I k l v m r R B K t x y S L w E e Y X";
const RESERVED_WORDS: &str = "f C I k l v m r R B K t x y S L w E e Y X";
const BODY_PARENTS: [&str; 6] = ["if_statement", "else_clause", "for_statement", "for_in_statement", "while_statement", "do_statement"];
const NO_HIDE_PARENTS: [&str; 5] = ["import_statement", "export_statement", "expression_statement", "literal_type", "import_require_clause"];
const NO_HIDE_ANCESTORS: [&str; 7] = [
    "type_annotation", "type_alias_declaration", "interface_declaration", "literal_type",
    "import_statement", "export_statement", "ambient_declaration",
];

fn is_ident(k: &str) -> bool { IDENT.contains(&k) }

pub use vtok_core::lang::{EncodeOpts, Encoded, View};

pub fn parse(src: &[u8]) -> Tree {
    let mut p = Parser::new();
    p.set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()).expect("ts grammar");
    let t = p.parse(src, None).expect("parse");
    if !t.root_node().has_error() { return t; }
    // JSX / TSX: retry with the TSX grammar, keep it only if it parses cleanly.
    p.set_language(&tree_sitter_typescript::LANGUAGE_TSX.into()).expect("tsx grammar");
    match p.parse(src, None) {
        Some(x) if !x.root_node().has_error() => x,
        _ => t,
    }
}

pub fn first_error<'a>(n: Node<'a>) -> Option<Node<'a>> {
    if n.kind() == "ERROR" || n.is_missing() { return Some(n); }
    if !n.has_error() { return None; }
    let mut c = n.walk();
    for ch in n.children(&mut c) {
        if let Some(e) = first_error(ch) { return Some(e); }
    }
    Some(n)
}

fn text<'a>(n: Node, src: &'a [u8]) -> &'a str { std::str::from_utf8(&src[n.start_byte()..n.end_byte()]).unwrap_or("") }

fn children<'a>(n: Node<'a>) -> Vec<Node<'a>> { let mut c = n.walk(); n.children(&mut c).collect() }
fn named_children<'a>(n: Node<'a>) -> Vec<Node<'a>> { let mut c = n.walk(); n.named_children(&mut c).collect() }

// ---------------------------------------------------------------- emission

struct Ctx<'a> {
    ren: &'a BTreeMap<String, String>,
    skeleton: bool,
    lean: bool,
    lits: &'a HashMap<usize, String>,
    src: &'a [u8],
}

fn isw(c: char) -> bool { c.is_alphanumeric() || c == '_' || c == '$' }

fn need_space(a: &str, b: &str) -> bool {
    let (Some(x), Some(y)) = (a.chars().last(), b.chars().next()) else { return false };
    if isw(x) && isw(y) { return true; }
    if x == y && (x == '+' || x == '-') { return true; }
    if x == '/' && (y == '/' || y == '*') { return true; }
    if x == '?' && y == '.' { return true; }
    if x == '!' && y == '=' { return true; }
    if !a.is_empty() && a.chars().all(|c| c.is_ascii_digit()) && y == '.' { return true; }
    false
}

fn join(tokens: &[String]) -> String {
    let mut out = String::new();
    for (i, t) in tokens.iter().enumerate() {
        if i > 0 && need_space(&tokens[i - 1], t) { out.push(' '); }
        out.push_str(t);
    }
    out
}

fn flat(n: Node, ctx: &Ctx) -> Result<Vec<String>> {
    let t = n.kind();
    if t == "comment" { return Ok(vec![]); }
    if t == "string" {
        if let Some(p) = ctx.lits.get(&n.start_byte()) { return Ok(vec![p.clone()]); }
    }
    if ctx.lean {
        if t == "type_annotation" { return Ok(vec![]); }
        if t == "import_statement" {
            let mut ids: Vec<String> = vec![];
            let mut st = vec![n];
            while let Some(x) = st.pop() {
                let ch = children(x);
                for c in ch.into_iter().rev() { st.push(c); }
                if x.kind() == "identifier" { ids.push(text(x, ctx.src).to_string()); }
            }
            return Ok(if ids.is_empty() { vec![] } else { vec![format!("import{{{}}}", ids.join(",")), ";".to_string()] });
        }
    }
    if is_ident(t) {
        let s = text(n, ctx.src);
        return Ok(vec![ctx.ren.get(s).cloned().unwrap_or_else(|| s.to_string())]);
    }
    if t == "template_string" { return Ok(vec![template(n, ctx)?]); }
    if ctx.skeleton && matches!(t, "arrow_function" | "function_expression" | "function" | "generator_function" | "function_declaration" | "generator_function_declaration" | "method_definition") {
        if let Some(b) = n.child_by_field_name("body") {
            if b.kind() == "statement_block" {
                let mut out = vec![];
                for c in children(n) {
                    if c.start_byte() < b.start_byte() && c.kind() != "comment" { out.extend(flat(c, ctx)?); }
                }
                out.push("{}".to_string());
                return Ok(out);
            }
        }
    }
    if t == "template_literal_type" {
        let mut parts = String::new();
        for c in children(n) {
            if c.kind() == "template_type" {
                let mut inner = vec![];
                for ch in children(c) {
                    if ch.kind() != "${" && ch.kind() != "}" { inner.extend(flat(ch, ctx)?); }
                }
                parts.push_str("${");
                parts.push_str(&join(&inner));
                parts.push('}');
            } else {
                parts.push_str(text(c, ctx.src));
            }
        }
        return Ok(vec![parts]);
    }
    if t == "jsx_text" {
        let j = jsx_norm(text(n, ctx.src));
        return Ok(if j.is_empty() { vec![] } else { vec![j] });
    }
    if n.child_count() == 0 || t == "string" || t == "regex" {
        let s = text(n, ctx.src);
        if s.contains('\n') { return Err(VtokError::new(format!("unsupported: newline inside {t}"))); }
        return Ok(vec![s.to_string()]);
    }
    let mut out = vec![];
    for c in children(n) { out.extend(flat(c, ctx)?); }
    // Source relying on ASI (no `;`): inline blocks need an explicit separator.
    if TERMINATED.contains(&t) && out.last().map_or(false, |l| l != ";") && !text(n, ctx.src).ends_with(';') {
        out.push(";".to_string());
    }
    if MEMBERS.contains(&t) && n.next_sibling().map_or(false, |s| s.kind() != ";" && s.kind() != ",") {
        out.push(";".to_string());
    }
    Ok(out)
}

const TERMINATED: &[&str] = &["expression_statement", "lexical_declaration", "variable_declaration", "return_statement", "throw_statement", "break_statement", "continue_statement", "debugger_statement", "do_statement", "public_field_definition", "type_alias_declaration", "import_statement"];

const MEMBERS: &[&str] = &["property_signature", "method_signature", "call_signature", "construct_signature", "index_signature", "abstract_method_signature"];

/// JSX whitespace semantics: lines are trimmed (first keeps leading, last keeps trailing space),
/// empty lines dropped, the rest joined by one space. Text without a newline is unchanged.
fn jsx_norm(s: &str) -> String {
    if !s.contains('\n') { return s.to_string(); }
    let lines: Vec<&str> = s.split('\n').collect();
    let last = lines.len() - 1;
    let mut parts: Vec<&str> = vec![];
    for (i, l) in lines.iter().enumerate() {
        let mut l = *l;
        if i != 0 { l = l.trim_start_matches([' ', '\t', '\r']); }
        if i != last { l = l.trim_end_matches([' ', '\t', '\r']); }
        if !l.is_empty() { parts.push(l); }
    }
    parts.join(" ")
}

fn template(n: Node, ctx: &Ctx) -> Result<String> {
    let tagged = n.parent().map_or(false, |p| p.kind() == "call_expression");
    let mut parts = String::new();
    for c in children(n) {
        if c.kind() == "template_substitution" {
            let mut inner = vec![];
            for ch in children(c) {
                if ch.kind() != "${" && ch.kind() != "}" { inner.extend(flat(ch, ctx)?); }
            }
            parts.push_str("${");
            parts.push_str(&join(&inner));
            parts.push('}');
        } else {
            let s = text(c, ctx.src);
            if s.contains('\n') {
                if tagged { return Err(VtokError::new("unsupported: multiline tagged template")); }
                parts.push_str(&s.replace('\n', "\\n"));
            } else {
                parts.push_str(s);
            }
        }
    }
    Ok(parts)
}

fn flat_stmt(n: Node, ctx: &Ctx) -> Result<Vec<String>> {
    let toks = flat(n, ctx)?;
    let mut out = Vec::with_capacity(toks.len());
    for i in 0..toks.len() {
        if toks[i] == ";" && i + 1 < toks.len() && toks[i + 1] == "}" { continue; }
        out.push(toks[i].clone());
    }
    Ok(out)
}

fn native_lines(nodes: &[Node], ctx: &Ctx) -> Result<Vec<String>> {
    let mut lines = vec![];
    for c in nodes {
        if SKIP.contains(&c.kind()) { continue; }
        lines.push(join(&flat_stmt(*c, ctx)?));
    }
    for i in 0..lines.len() {
        let nxt: String = if i + 1 < lines.len() { lines[i + 1].clone() } else { String::new() };
        if lines[i].ends_with(';') {
            let first = nxt.chars().next();
            if first.map_or(true, |c| c.is_alphabetic() || c == '_' || c == '$' || c == '@') {
                lines[i].pop();
            }
        }
    }
    Ok(lines)
}

// ---------------------------------------------------------------- analysis

fn idents(n: Node, src: &[u8], out: &mut HashSet<String>) {
    if is_ident(n.kind()) { out.insert(text(n, src).to_string()); }
    for c in children(n) { idents(c, src, out); }
}

pub struct Analysis {
    pub counts: HashMap<String, usize>,
    pub excl: HashSet<String>,
    pub avoid: HashSet<String>,
}

fn words(s: &str) -> impl Iterator<Item = String> + '_ { s.split_whitespace().map(String::from) }

pub fn analyze(root: Node, src: &[u8]) -> Analysis {
    let mut counts: HashMap<String, usize> = HashMap::new();
    let mut excl: HashSet<String> = HashSet::new();
    let mut avoid: HashSet<String> = words(JS_WORDS).chain(words(KW_VALUES)).collect();
    avoid.insert("?".into());
    avoid.insert("e".into());
    let mut st = vec![root];
    while let Some(n) = st.pop() {
        for c in children(n) { st.push(c); }
        let t = n.kind();
        if LIKE_IDENT.contains(&t) { avoid.insert(text(n, src).to_string()); }
        if is_ident(t) {
            *counts.entry(text(n, src).to_string()).or_insert(0) += 1;
        } else if t == "shorthand_property_identifier" || t == "shorthand_property_identifier_pattern" {
            excl.insert(text(n, src).to_string());
        } else if t == "import_statement" {
            idents(n, src, &mut excl);
        } else if t == "export_statement" {
            for c in children(n) {
                if is_ident(c.kind()) { excl.insert(text(c, src).to_string()); }
                else if c.kind() == "export_clause" { idents(c, src, &mut excl); }
            }
            if let Some(d) = n.child_by_field_name("declaration") {
                if let Some(nm) = d.child_by_field_name("name") { idents(nm, src, &mut excl); }
                for vd in named_children(d) {
                    if vd.kind() == "variable_declarator" {
                        if let Some(nm) = vd.child_by_field_name("name") { idents(nm, src, &mut excl); }
                    }
                }
            }
        }
    }
    Analysis { counts, excl, avoid }
}

fn collect_literals(root: Node, src: &[u8], minimum: usize) -> (HashMap<usize, String>, BTreeMap<String, String>) {
    let mut strings: Vec<Node> = vec![];
    let mut existing: HashSet<String> = HashSet::new();
    let mut st = vec![root];
    while let Some(n) = st.pop() {
        for c in children(n) { st.push(c); }
        if n.kind() == "string" {
            existing.insert(text(n, src).to_string());
            let mut p = n.parent();
            let mut ok = true;
            while let Some(a) = p {
                let k = a.kind();
                let is_direct = n.parent().map_or(false, |q| q.id() == a.id());
                if NO_HIDE_ANCESTORS.contains(&k) && !(k == "export_statement" && !is_direct) { ok = false; break; }
                p = a.parent();
            }
            let par_ok = n.parent().map_or(true, |q| !NO_HIDE_PARENTS.contains(&q.kind()));
            if ok && par_ok && ntok(text(n, src)) >= minimum { strings.push(n); }
        }
    }
    strings.sort_by_key(|n| n.start_byte());
    let (mut pos, mut back) = (HashMap::new(), BTreeMap::new());
    let mut by_text: HashMap<String, String> = HashMap::new();
    let mut k = 0usize;
    for n in strings {
        let s = text(n, src).to_string();
        if !by_text.contains_key(&s) {
            k += 1;
            while existing.contains(&format!("\"${k}\"")) { k += 1; }
            let ph = format!("\"${k}\"");
            by_text.insert(s.clone(), ph.clone());
            back.insert(ph, s.clone());
        }
        pos.insert(n.start_byte(), by_text[&s].clone());
    }
    (pos, back)
}

// ---------------------------------------------------------------- canonical dump

fn canon(n: Node, src: &[u8], out: &mut Vec<String>) {
    let t = n.kind();
    if SKIP.contains(&t) { return; }
    if !n.is_named() {
        if t != ";" && t != "," { out.push(t.to_string()); }
        return;
    }
    if t == "statement_block" && n.parent().map_or(false, |p| BODY_PARENTS.contains(&p.kind())) {
        let kids: Vec<Node> = named_children(n).into_iter().filter(|c| !SKIP.contains(&c.kind())).collect();
        if kids.len() == 1 { return canon(kids[0], src, out); }
    }
    if t == "template_string" {
        out.push("(template".into());
        let mut buf = String::new();
        let mut has = false;
        let flush = |buf: &mut String, has: &mut bool, out: &mut Vec<String>| {
            if *has { out.push(format!("txt:{buf}")); buf.clear(); *has = false; }
        };
        for c in children(n) {
            match c.kind() {
                "string_fragment" | "escape_sequence" => { buf.push_str(&text(c, src).replace('\n', "\\n")); has = true; }
                "template_substitution" => { flush(&mut buf, &mut has, out); canon(c, src, out); }
                _ => {}
            }
        }
        flush(&mut buf, &mut has, out);
        out.push(")".into());
        return;
    }
    if t == "jsx_text" {
        let j = jsx_norm(text(n, src));
        if !j.is_empty() { out.push(format!("{t}:{j}")); }
        return;
    }
    if n.child_count() == 0 || t == "string" || t == "regex" {
        out.push(format!("{t}:{}", text(n, src)));
        return;
    }
    out.push(format!("({t}"));
    for c in children(n) { canon(c, src, out); }
    out.push(")".into());
}

// ---------------------------------------------------------------- public API

pub fn encode(source: &str, opts: &EncodeOpts) -> Result<Encoded> {
    let source = source.replace("\r\n", "\n");
    let src = source.as_bytes();
    let tree = parse(src);
    let root = tree.root_node();
    if let Some(e) = first_error(root) {
        return Err(VtokError::new(format!("source has syntax errors near line {}", e.start_position().row + 1)));
    }
    let an = analyze(root, src);
    let reserved: HashSet<String> = words(RESERVED_WORDS).collect();
    let table: BTreeMap<String, String> = if let Some(t) = &opts.table {
        t.clone()
    } else if opts.alias {
        plan_aliases(&an.counts, &an.avoid, &reserved, &an.excl, 2)
    } else {
        let bad: HashMap<String, usize> = an.counts.keys().filter(|n| reserved.contains(*n)).map(|n| (n.clone(), 1)).collect();
        if bad.is_empty() { BTreeMap::new() } else { plan_aliases(&bad, &HashSet::new(), &reserved, &HashSet::new(), 2) }
    };
    let (pos, lits) = if opts.hide_literals > 0 { collect_literals(root, src, opts.hide_literals) } else { (HashMap::new(), BTreeMap::new()) };
    let ctx = Ctx { ren: &table, skeleton: opts.view == View::Skeleton, lean: opts.view == View::Lean, lits: &pos, src };
    let top = named_children(root);
    let mut vt = native_lines(&top, &ctx)?.join("\n");
    vt.push('\n');
    if opts.verify && opts.view != View::Lean {
        let inv: HashMap<String, String> = table.iter().map(|(o, a)| (a.clone(), o.clone())).collect();
        let back = decode(&vt, &inv, &lits.iter().map(|(k, v)| (k.clone(), v.clone())).collect())?;
        if opts.view == View::Full {
            let bt = parse(back.as_bytes());
            let (mut a, mut b) = (vec![], vec![]);
            canon(bt.root_node(), back.as_bytes(), &mut a);
            canon(root, src, &mut b);
            if a != b { return Err(VtokError::new("roundtrip verification failed (tree differs)")); }
        }
    }
    Ok(Encoded { vtok: vt, table, literals: lits })
}

/// table: {alias: original}; literals: {placeholder: original text}.
pub fn decode(vtok: &str, table: &HashMap<String, String>, literals: &HashMap<String, String>) -> Result<String> {
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
    let mut st = vec![root];
    while let Some(n) = st.pop() {
        for c in children(n) { st.push(c); }
        let t = n.kind();
        let s = text(n, raw);
        if t == "string" && literals.contains_key(s) {
            spans.push((n.start_byte(), n.end_byte(), literals[s].clone()));
        } else if is_ident(t) && table.contains_key(s) {
            spans.push((n.start_byte(), n.end_byte(), table[s].clone()));
        }
    }
    spans.sort_by(|a, b| b.0.cmp(&a.0));
    let mut out = raw.to_vec();
    for (a, b, r) in spans { out.splice(a..b, r.into_bytes()); }
    Ok(String::from_utf8_lossy(&out).into_owned())
}

mod plugin;
pub use plugin::TypeScript;
