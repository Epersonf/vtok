//! Rust plugin: native minified notation (tree-sitter). Every token is kept (`;` and `,` included), only
//! comments and optional whitespace go away; identifiers are aliased through the table.
use std::collections::{BTreeMap, HashMap, HashSet};
use tree_sitter::{Node, Parser, Tree};
use vtok_core::aliasing::plan_aliases;
use vtok_core::tokens::ntok;
use vtok_core::{Result, VtokError};

pub use vtok_core::lang::{EncodeOpts, Encoded, View};

const IDENT: [&str; 4] = ["identifier", "type_identifier", "field_identifier", "shorthand_field_identifier"];
const COMMENTS: [&str; 2] = ["line_comment", "block_comment"];
const SKIP: [&str; 4] = ["line_comment", "block_comment", "empty_statement", "shebang"];
const STRINGS: [&str; 2] = ["string_literal", "raw_string_literal"];
const LEAF_TEXT: [&str; 3] = ["string_literal", "raw_string_literal", "char_literal"];
const RUST_WORDS: &str = "abstract as async await become box break const continue crate do dyn else enum extern false final fn for gen if impl in let loop macro match mod move mut override priv pub ref return self Self static struct super trait true try type typeof union unsafe unsized use virtual where while yield bool char str u8 u16 u32 u64 u128 usize i8 i16 i32 i64 i128 isize f32 f64 Some None Ok Err Option Result Vec String Box Default main r b br";
const NO_HIDE_ANCESTORS: [&str; 5] = ["attribute_item", "inner_attribute_item", "use_declaration", "extern_crate_declaration", "foreign_mod_item"];
const OPS2: &[&str] = &[
    "&&", "||", "<<", ">>", "<=", ">=", "==", "!=", "->", "=>", "::", "..", "+=", "-=", "*=", "/=", "%=", "^=", "&=", "|=", "<-", "/*", "//", "*/",
];
const OPS3: &[&str] = &["..=", "...", "<<=", ">>="];
/// Wrapper used to parse member chunks (items taken out of an impl / trait / mod body).
const WRAP: &str = "__vtok_wrap__";

pub fn parse(src: &[u8]) -> Tree {
    let mut p = Parser::new();
    p.set_language(&tree_sitter_rust::LANGUAGE.into()).expect("rust grammar");
    match mask_raw_ident(src) {
        Some(m) => p.parse(&m, None).expect("parse"),
        None => p.parse(src, None).expect("parse"),
    }
}

/// tree-sitter-rust reads `&raw` as the start of `&raw const/mut`, so a variable named `raw`
/// (`f(&raw)`) fails to parse. Mask such an identifier with one of equal length (offsets stay valid).
fn mask_raw_ident(src: &[u8]) -> Option<Vec<u8>> {
    let mut out: Option<Vec<u8>> = None;
    let mut i = 0;
    while i + 4 <= src.len() {
        if &src[i..i + 3] == b"raw" && (i == 0 || src[i - 1] == b'&' || src[i - 1].is_ascii_whitespace()) {
            let mut b = i;
            while b > 0 && src[b - 1].is_ascii_whitespace() { b -= 1; }
            let after = src[i + 3];
            let ident_end = after.is_ascii_alphanumeric() || after == b'_';
            if b > 0 && src[b - 1] == b'&' && !ident_end {
                let mut j = i + 3;
                while j < src.len() && src[j].is_ascii_whitespace() { j += 1; }
                let rest = &src[j..];
                let kw = (rest.starts_with(b"const") || rest.starts_with(b"mut"))
                    && !rest.get(if rest.starts_with(b"const") { 5 } else { 3 }).map_or(false, |c| c.is_ascii_alphanumeric() || *c == b'_');
                if !kw { out.get_or_insert_with(|| src.to_vec())[i + 2] = b'x'; }
            }
        }
        i += 1;
    }
    out
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
fn words(s: &str) -> impl Iterator<Item = String> + '_ { s.split_whitespace().map(String::from) }

/// An identifier that the alias table may rename (lifetimes and labels keep their names).
fn renamable(n: Node) -> bool {
    IDENT.contains(&n.kind()) && n.parent().map_or(true, |p| !matches!(p.kind(), "lifetime" | "label"))
}

// ---------------------------------------------------------------- emission

struct Ctx<'a> {
    ren: &'a BTreeMap<String, String>,
    skeleton: bool,
    lean: bool,
    lits: &'a HashMap<usize, String>,
    src: &'a [u8],
}

fn isw(c: char) -> bool { c.is_alphanumeric() || c == '_' }

fn need_space(a: &str, b: &str) -> bool {
    let (Some(x), Some(y)) = (a.chars().last(), b.chars().next()) else { return false };
    if isw(x) && isw(y) { return true; }
    if !isw(x) && !isw(y) {
        let two = format!("{x}{y}");
        if OPS2.contains(&two.as_str()) { return true; }
        let ac: Vec<char> = a.chars().collect();
        let bc: Vec<char> = b.chars().collect();
        if ac.len() >= 2 && OPS3.contains(&format!("{}{x}{y}", ac[ac.len() - 2]).as_str()) { return true; }
        if bc.len() >= 2 && OPS3.contains(&format!("{x}{y}{}", bc[1]).as_str()) { return true; }
    }
    if !a.is_empty() && a.chars().all(|c| c.is_ascii_digit() || c == '_') && y == '.' && !b.starts_with("..") { return true; }
    if x == '.' && y.is_ascii_digit() && a.starts_with(|c: char| c.is_ascii_digit()) { return true; }
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

fn is_name(p: Option<Node>, n: Node, field: &str) -> bool {
    p.and_then(|p| p.child_by_field_name(field)).map_or(false, |x| x.id() == n.id())
}

/// Identifiers a `use` keeps in the lean view: the last segment of every path.
fn use_names(n: Node, ctx: &Ctx, out: &mut Vec<String>) {
    if n.kind() == "identifier" {
        let p = n.parent();
        let is_path = p.map_or(false, |p| matches!(p.kind(), "scoped_identifier" | "scoped_use_list") && is_name(Some(p), n, "path"));
        if !is_path {
            let s = text(n, ctx.src);
            out.push(ctx.ren.get(s).cloned().unwrap_or_else(|| s.to_string()));
        }
    }
    for c in children(n) { use_names(c, ctx, out); }
}

fn flat(n: Node, ctx: &Ctx) -> Result<Vec<String>> {
    let t = n.kind();
    if COMMENTS.contains(&t) { return Ok(vec![]); }
    if STRINGS.contains(&t) {
        if let Some(p) = ctx.lits.get(&n.start_byte()) { return Ok(vec![p.clone()]); }
    }
    if LEAF_TEXT.contains(&t) { return Ok(vec![text(n, ctx.src).to_string()]); }
    if renamable(n) {
        let s = text(n, ctx.src);
        return Ok(vec![ctx.ren.get(s).cloned().unwrap_or_else(|| s.to_string())]);
    }
    if ctx.lean {
        if t == "use_declaration" {
            let mut ids = vec![];
            use_names(n, ctx, &mut ids);
            return Ok(if ids.is_empty() { vec![] } else { vec![format!("use{{{}}}", ids.join(",")), ";".to_string()] });
        }
        if matches!(t, "let_declaration" | "parameter" | "function_item") {
            let drop = n.child_by_field_name(if t == "function_item" { "return_type" } else { "type" });
            if let Some(d) = drop {
                let kids = children(n);
                let mut out = vec![];
                for (i, c) in kids.iter().enumerate() {
                    let before_drop = kids.get(i + 1).map_or(false, |k| k.id() == d.id()) && matches!(c.kind(), ":" | "->");
                    if c.id() == d.id() || before_drop { continue; }
                    out.extend(flat(*c, ctx)?);
                }
                return Ok(out);
            }
        }
    }
    if ctx.skeleton && t == "function_item" {
        if let Some(b) = n.child_by_field_name("body") {
            let mut out = vec![];
            for c in children(n) {
                if c.start_byte() < b.start_byte() { out.extend(flat(c, ctx)?); }
            }
            out.push("{}".to_string());
            return Ok(out);
        }
    }
    if n.child_count() == 0 { return Ok(vec![text(n, ctx.src).to_string()]); }
    let mut out = vec![];
    for c in children(n) { out.extend(flat(c, ctx)?); }
    Ok(out)
}

fn native_lines(nodes: &[Node], ctx: &Ctx) -> Result<Vec<String>> {
    let mut lines = vec![];
    for c in nodes {
        if SKIP.contains(&c.kind()) { continue; }
        lines.push(join(&flat(*c, ctx)?));
    }
    Ok(lines)
}

// ---------------------------------------------------------------- analysis

pub struct Analysis {
    pub counts: HashMap<String, usize>,
    pub excl: HashSet<String>,
    pub avoid: HashSet<String>,
}

fn add_idents(n: Node, src: &[u8], out: &mut HashSet<String>) {
    if IDENT.contains(&n.kind()) { out.insert(text(n, src).to_string()); }
    for c in children(n) { add_idents(c, src, out); }
}

fn add_name(n: Node, field: &str, src: &[u8], out: &mut HashSet<String>) {
    if let Some(x) = n.child_by_field_name(field) { add_idents(x, src, out); }
}

fn has_vis(n: Node) -> bool { children(n).iter().any(|c| c.kind() == "visibility_modifier") }

/// `{name}` / `{name:?}` captures inside format strings refer to variables by their real name.
fn format_captures(s: &str, out: &mut HashSet<String>) {
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'{' {
            let st = i + 1;
            let mut j = st;
            while j < b.len() && (b[j].is_ascii_alphanumeric() || b[j] == b'_') { j += 1; }
            if j > st && !b[st].is_ascii_digit() { out.insert(s[st..j].to_string()); }
            i = j.max(i + 1);
        } else { i += 1; }
    }
}

pub fn analyze(root: Node, src: &[u8]) -> Analysis {
    let mut counts: HashMap<String, usize> = HashMap::new();
    let mut excl: HashSet<String> = HashSet::new();
    let mut avoid: HashSet<String> = words(RUST_WORDS).collect();
    avoid.insert("e".into());
    let mut st = vec![root];
    while let Some(n) = st.pop() {
        for c in children(n) { st.push(c); }
        let t = n.kind();
        if n.child_count() == 0 || IDENT.contains(&t) {
            let s = text(n, src);
            if s.chars().next().map_or(false, |c| c.is_alphabetic() || c == '_') && s.chars().all(isw) { avoid.insert(s.to_string()); }
        }
        if renamable(n) {
            let s = text(n, src);
            *counts.entry(s.to_string()).or_insert(0) += 1;
            if s.starts_with("r#") { excl.insert(s.to_string()); }
        }
        match t {
            "string_literal" => format_captures(text(n, src), &mut excl),
            "use_declaration" | "extern_crate_declaration" | "attribute_item" | "inner_attribute_item" | "foreign_mod_item" => add_idents(n, src, &mut excl),
            "macro_invocation" => add_name(n, "macro", src, &mut excl),
            "macro_definition" | "mod_item" => add_name(n, "name", src, &mut excl),
            "function_item" | "struct_item" | "enum_item" | "trait_item" | "const_item" | "static_item" | "type_item" | "union_item" | "field_declaration" | "function_signature_item" => {
                if has_vis(n) || t == "trait_item" { add_name(n, "name", src, &mut excl); }
                if t == "enum_item" && has_vis(n) {
                    if let Some(b) = n.child_by_field_name("body") {
                        for v in named_children(b) { if v.kind() == "enum_variant" { add_name(v, "name", src, &mut excl); } }
                    }
                }
                if t == "trait_item" {
                    if let Some(b) = n.child_by_field_name("body") {
                        for m in named_children(b) { add_name(m, "name", src, &mut excl); }
                    }
                }
            }
            "impl_item" => {
                if n.child_by_field_name("trait").is_some() {
                    if let Some(b) = n.child_by_field_name("body") {
                        for m in named_children(b) { add_name(m, "name", src, &mut excl); }
                    }
                }
            }
            _ => {}
        }
    }
    excl.insert("Self".into());
    Analysis { counts, excl, avoid }
}

fn collect_literals(root: Node, src: &[u8], minimum: usize) -> (HashMap<usize, String>, BTreeMap<String, String>) {
    let mut strings: Vec<Node> = vec![];
    let mut existing: HashSet<String> = HashSet::new();
    let mut st = vec![root];
    while let Some(n) = st.pop() {
        for c in children(n) { st.push(c); }
        if STRINGS.contains(&n.kind()) {
            existing.insert(text(n, src).to_string());
            let mut p = n.parent();
            let mut ok = true;
            while let Some(a) = p {
                if NO_HIDE_ANCESTORS.contains(&a.kind()) { ok = false; break; }
                p = a.parent();
            }
            if ok && ntok(text(n, src)) >= minimum { strings.push(n); }
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
        out.push(t.to_string());
        return;
    }
    if (n.child_count() == 0 && t != "source_file") || LEAF_TEXT.contains(&t) {
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
    let table: BTreeMap<String, String> = if let Some(t) = &opts.table {
        t.clone()
    } else if opts.alias {
        plan_aliases(&an.counts, &an.avoid, &HashSet::new(), &an.excl, 2)
    } else {
        BTreeMap::new()
    };
    let (pos, lits) = if opts.hide_literals > 0 { collect_literals(root, src, opts.hide_literals) } else { (HashMap::new(), BTreeMap::new()) };
    let ctx = Ctx { ren: &table, skeleton: opts.view == View::Skeleton, lean: opts.view == View::Lean, lits: &pos, src };
    let top = named_children(root);
    let mut vt = native_lines(&top, &ctx)?.join("\n");
    vt.push('\n');
    if opts.verify && opts.view == View::Full {
        let inv: HashMap<String, String> = table.iter().map(|(o, a)| (a.clone(), o.clone())).collect();
        let back = decode(&vt, &inv, &lits.iter().map(|(k, v)| (k.clone(), v.clone())).collect())?;
        let bt = parse(back.as_bytes());
        let (mut a, mut b) = (vec![], vec![]);
        canon(bt.root_node(), back.as_bytes(), &mut a);
        canon(root, src, &mut b);
        if a != b {
            let i = a.iter().zip(b.iter()).position(|(x, y)| x != y).unwrap_or(0);
            let ctx = |v: &Vec<String>| v[i.saturating_sub(4)..(i + 3).min(v.len())].join(" ");
            return Err(VtokError::new(format!("roundtrip verification failed (tree differs): got [{}] want [{}]", ctx(&a), ctx(&b))));
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
        let s = text(n, raw);
        if STRINGS.contains(&n.kind()) && literals.contains_key(s) {
            spans.push((n.start_byte(), n.end_byte(), literals[s].clone()));
        } else if renamable(n) && table.contains_key(s) {
            spans.push((n.start_byte(), n.end_byte(), table[s].clone()));
        }
    }
    spans.sort_by(|a, b| b.0.cmp(&a.0));
    let mut out = raw.to_vec();
    for (a, b, r) in spans { out.splice(a..b, r.into_bytes()); }
    Ok(String::from_utf8_lossy(&out).into_owned())
}

mod plugin;
pub use plugin::Rust;
