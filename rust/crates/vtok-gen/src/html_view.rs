//! HTML views. Outline: landmarks as `tag#id.class` (html, head, body, nav, main, section, form, table...), headings
//! with their text, `script[src=...]` and `link[href=...]`. Skeleton: the element tree with tag, id, class, name, href,
//! src and type, no text except short headings and titles, inline formatting dropped, repeated siblings as `×N`.
//! Lean: the full view with inline script and style bodies and very long attribute values replaced by a size marker.
use crate::emit::Emit;
use crate::spec::Spec;
use crate::tree::*;
use std::collections::BTreeMap;
use tree_sitter::Node;

const LANDMARKS: &[&str] = &["html", "head", "body", "header", "footer", "nav", "main", "section", "article", "aside", "form", "table", "template", "dialog", "iframe"];
const INLINE: &[&str] = &["b", "i", "em", "strong", "span", "br", "wbr", "small", "sup", "sub", "u", "s", "font", "mark", "abbr", "code", "kbd", "var", "tt", "img"];
const KEEP: &[&str] = &["id", "class", "name", "href", "src", "type", "action", "for", "role"];

pub fn is_element(n: Node) -> bool { matches!(n.kind(), "element" | "script_element" | "style_element") }

fn tag_node<'a>(n: Node<'a>) -> Option<Node<'a>> { children(n).into_iter().find(|c| c.kind() == "start_tag" || c.kind() == "self_closing_tag") }

pub fn tag(n: Node, src: &[u8]) -> String {
    tag_node(n).and_then(|t| children(t).into_iter().find(|c| c.kind() == "tag_name")).map(|t| text(t, src).to_ascii_lowercase()).unwrap_or_default()
}

/// (name, value) of the attributes of `n`'s start tag.
pub fn attrs(n: Node, src: &[u8]) -> Vec<(String, String)> {
    let Some(t) = tag_node(n) else { return vec![] };
    children(t).into_iter().filter(|a| a.kind() == "attribute").map(|a| {
        let k = children(a).into_iter().find(|c| c.kind() == "attribute_name").map(|c| text(c, src).to_ascii_lowercase()).unwrap_or_default();
        let v = children(a).into_iter().find(|c| c.kind().contains("attribute_value")).map(|c| text(c, src).trim_matches(['"', '\'']).to_string()).unwrap_or_default();
        (k, v)
    }).collect()
}

fn short(s: &str, n: usize) -> String { let s = s.split_whitespace().collect::<Vec<_>>().join(" "); if s.chars().count() > n { s.chars().take(n).collect::<String>() + "…" } else { s } }

/// `tag#id.class1.class2`, plus `[src=..]` / `[href=..]` for scripts and links.
pub fn label(n: Node, src: &[u8]) -> String {
    let t = tag(n, src);
    let mut s = t.clone();
    for (k, v) in attrs(n, src) {
        match k.as_str() {
            "id" => s += &format!("#{}", short(&v, 40)),
            "class" => for c in v.split_whitespace().take(3) { s += &format!(".{c}"); },
            "src" | "href" if matches!(t.as_str(), "script" | "link" | "iframe") => s += &format!("[{k}={}]", short(&v, 60)),
            _ => {}
        }
    }
    s
}

/// The direct text of `n` (headings, titles), whitespace collapsed.
fn own_text(n: Node, src: &[u8]) -> String {
    let mut out = String::new();
    for c in children(n) { if c.kind() == "text" { out.push_str(text(c, src)); out.push(' '); } else if is_element(c) { out.push_str(&own_text(c, src)); } }
    out
}

/// Landmarks for the outline, in document order (nested ones too).
pub fn landmarks<'a>(n: Node<'a>, src: &[u8], out: &mut Vec<(Node<'a>, String)>) {
    for c in named_children(n) {
        if !is_element(c) { if c.kind() == "ERROR" { landmarks(c, src, out); } continue; }
        let t = tag(c, src);
        let keyed = (t == "script" && attrs(c, src).iter().any(|a| a.0 == "src")) || (t == "link" && attrs(c, src).iter().any(|a| a.0 == "href"));
        if LANDMARKS.contains(&t.as_str()) || keyed { out.push((c, label(c, src))); }
        if matches!(t.as_str(), "h1" | "h2" | "h3" | "title") { out.push((c, format!("{t} {}", short(&own_text(c, src), 50)))); }
        landmarks(c, src, out);
    }
}

fn render(n: Node, src: &[u8]) -> String {
    let t = tag(n, src);
    let mut s = format!("<{t}");
    for (k, v) in attrs(n, src) { if KEEP.contains(&k.as_str()) { s += &if v.is_empty() { format!(" {k}") } else { format!(" {k}={}", short(&v, 60)) }; } }
    s.push('>');
    if matches!(t.as_str(), "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "title" | "button" | "label" | "option" | "legend" | "caption") { s += &short(&own_text(n, src), 60); }
    let kids = body(n, src);
    if kids.is_empty() && !s.ends_with('>') { return s + &format!("</{t}>"); }
    if kids.is_empty() { return s; }
    s + &kids + &format!("</{t}>")
}

/// The rendered child elements of `n`, inline formatting dropped, runs of identical siblings collapsed to `×N`.
fn body(n: Node, src: &[u8]) -> String {
    let mut items: Vec<(String, usize)> = vec![];
    for c in named_children(n) {
        let r = if is_element(c) && INLINE.contains(&tag(c, src).as_str()) && !attrs(c, src).iter().any(|a| a.0 == "id") { body(c, src) }
            else if is_element(c) { render(c, src) } else if c.kind() == "ERROR" { body(c, src) } else { continue };
        if r.is_empty() { continue; }
        match items.last_mut() { Some(l) if l.0 == r => l.1 += 1, _ => items.push((r, 1)) }
    }
    let block = |r: &str| ["<div", "<section", "<nav", "<main", "<header", "<footer", "<form", "<table", "<ul", "<ol", "<article", "<aside", "<head", "<body"].iter().any(|b| r.starts_with(b));
    items.into_iter().map(|(r, k)| format!("{}{r}{}", if block(&r) { "\n" } else { "" }, if k > 1 { format!("×{k}") } else { String::new() })).collect()
}

pub fn skeleton(src: &[u8], root: Node) -> String {
    let mut s = body(root, src).trim_start().to_string();
    s.push('\n');
    s
}

fn lean_marks(n: Node, src: &[u8], em: &mut Emit) {
    if n.kind() == "raw_text" && n.end_byte() - n.start_byte() > 200 {
        let lines = text(n, src).lines().filter(|l| !l.trim().is_empty()).count();
        em.subst.insert(n.id(), format!("/* … {lines} lines, {} chars */", n.end_byte() - n.start_byte()));
    }
    if n.kind() == "attribute_value" && n.end_byte() - n.start_byte() > 120 { em.subst.insert(n.id(), format!("…{} chars", n.end_byte() - n.start_byte())); }
    for c in children(n) { lean_marks(c, src, em); }
}

pub fn lean(spec: &'static Spec, src: &[u8], root: Node) -> String {
    let none = BTreeMap::new();
    let tops = named_children(root);
    let mut em = Emit::new(src, spec, &none, false, false, &tops);
    lean_marks(root, src, &mut em);
    em.run(&children(root))
}
