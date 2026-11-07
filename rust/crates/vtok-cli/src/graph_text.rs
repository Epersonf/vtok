//! Text helpers for document nodes: tag stripping, docx and subtitle text, and linking prose to code symbols.
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use super::Node;

const MAX_LINKS: usize = 200;

pub(crate) fn strip_tags(s: &str) -> String {
    let mut t = s.to_string();
    for tag in ["script", "style"] {
        let re = Regex::new(&format!(r"(?is)<{tag}\b.*?</{tag}>")).unwrap();
        t = re.replace_all(&t, " ").into_owned();
    }
    static TAG: OnceLock<Regex> = OnceLock::new();
    let t = TAG.get_or_init(|| Regex::new(r"(?s)<[^>]*>").unwrap()).replace_all(&t, " ");
    t.replace("&lt;", "<").replace("&gt;", ">").replace("&amp;", "&").replace("&quot;", "\"").replace("&nbsp;", " ")
}

/// Visible text of an SVG: its `<title>`, `<desc>` and `<text>` contents.
pub(crate) fn svg_text(raw: &str) -> String {
    static R: OnceLock<Regex> = OnceLock::new();
    let re = R.get_or_init(|| Regex::new(r"(?is)<(title|desc|text|tspan)\b[^>]*>(.*?)</(?:title|desc|text|tspan)>").unwrap());
    re.captures_iter(raw).map(|c| strip_tags(&c[2]).trim().to_string()).filter(|s| !s.is_empty()).collect::<Vec<_>>().join("\n")
}

/// The width and height an SVG declares (or its viewBox), in px.
pub(crate) fn svg_size(raw: &str) -> (usize, usize) {
    let head = raw.find("<svg").map_or("", |i| &raw[i..raw[i..].find('>').map_or(raw.len(), |j| i + j)]);
    let attr = |n: &str| -> Option<f64> {
        let re = Regex::new(&format!(r#"\b{n}\s*=\s*["']\s*([0-9.]+)"#)).unwrap();
        re.captures(head).and_then(|c| c[1].parse().ok())
    };
    let vb = Regex::new(r#"viewBox\s*=\s*["']\s*[-0-9.]+[ ,]+[-0-9.]+[ ,]+([0-9.]+)[ ,]+([0-9.]+)"#).unwrap().captures(head).map(|c| (c[1].parse().unwrap_or(0.0), c[2].parse().unwrap_or(0.0)));
    let (w, h) = (attr("width").or(vb.map(|v: (f64, f64)| v.0)), attr("height").or(vb.map(|v| v.1)));
    (w.unwrap_or(0.0) as usize, h.unwrap_or(0.0) as usize)
}

/// Paragraph text of a .docx (a zip with word/document.xml).
pub(crate) fn docx_text(path: &std::path::Path) -> Option<String> {
    use std::io::Read;
    let mut z = zip::ZipArchive::new(std::fs::File::open(path).ok()?).ok()?;
    let mut xml = String::new();
    z.by_name("word/document.xml").ok()?.take(20_000_000).read_to_string(&mut xml).ok()?;
    Some(strip_tags(&xml.replace("</w:p>", "\n").replace("<w:tab/>", " ")))
}

/// SubRip/WebVTT without cue numbers, timestamps and markup.
pub(crate) fn subtitle_text(raw: &str) -> String {
    let lines = raw.lines().map(str::trim).filter(|l| !l.is_empty() && !l.contains("-->") && *l != "WEBVTT" && !l.chars().all(|c| c.is_ascii_digit()));
    strip_tags(&lines.collect::<Vec<_>>().join("\n"))
}

fn identifier_looking(tok: &str, call: bool) -> bool {
    call || tok.contains('_') || tok.chars().skip(1).any(|c| c.is_uppercase())
}

/// Code symbols a text refers to, each with exactly one definition: names in backticks (`foo()`, `Type.method`) and
/// plain identifier-looking tokens (snake_case, camelCase, PascalCase, or followed by a call paren).
pub(crate) fn names_in(text: &str, nodes: &[Node], by_base: &HashMap<String, Vec<usize>>) -> Vec<usize> {
    let mut out: Vec<usize> = vec![];
    let mut seen: HashSet<usize> = HashSet::new();
    let mut add = |defs: Option<&Vec<usize>>, plain: bool| {
        if let Some(d) = defs { if d.len() == 1 && out.len() < MAX_LINKS && !(plain && nodes[d[0]].kind == "value") && seen.insert(d[0]) { out.push(d[0]); } }
    };
    for (i, seg) in text.split('`').enumerate() {
        if i % 2 == 0 { continue; }
        let w = seg.trim().trim_end_matches("()");
        add(by_base.get(w.rsplit(['.', ':']).next().unwrap_or(w)), false);
    }
    static TOK: OnceLock<Regex> = OnceLock::new();
    for m in TOK.get_or_init(|| Regex::new(r"[A-Za-z_][A-Za-z0-9_]{3,}\(?").unwrap()).find_iter(text) {
        let s = m.as_str();
        let tok = s.trim_end_matches('(');
        if identifier_looking(tok, s.ends_with('(')) { add(by_base.get(tok), true); }
    }
    out
}
