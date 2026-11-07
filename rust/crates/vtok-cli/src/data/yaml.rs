//! Block-style YAML as far as repeated records need it: nested mappings and sequences, plain and quoted scalars,
//! one-line flow collections and `|`/`>` text. Anchors, tags, merge keys, multi-line scalars, tabs and several
//! documents make the reader give up (None), and the file is shown some other way.
use super::yaml_scalar::value;
use serde_json::{Map, Value};

pub fn read(lines: &[String]) -> Option<super::records::Table> { super::json::from_value("yaml", &parse(lines)?) }

/// The document as a JSON value.
pub(super) fn parse(lines: &[String]) -> Option<Value> {
    let mut ls: Vec<(usize, String)> = vec![];
    for l in lines {
        let body = strip_comment(l).trim_end();
        let content = body.trim_start();
        if content.is_empty() { continue; }
        if body.starts_with('\t') || content == "..." || (content == "---" && !ls.is_empty()) { return None; }
        if content == "---" { continue; }
        ls.push((body.len() - content.len(), content.to_string()));
    }
    let mut p = P { ls, i: 0 };
    let v = p.block()?;
    (p.i == p.ls.len()).then_some(v)
}

/// The line without a trailing ` # comment` (a `#` inside quotes or glued to text is not one).
fn strip_comment(l: &str) -> &str {
    let b = l.as_bytes();
    let (mut single, mut double) = (false, false);
    for i in 0..b.len() {
        match b[i] {
            b'\'' if !double => single = !single,
            b'"' if !single => double = !double,
            b'#' if !single && !double && (i == 0 || b[i - 1] == b' ') => return &l[..i],
            _ => {}
        }
    }
    l
}

/// `key: rest` as (key, rest); None when the line is not a mapping entry.
fn split_key(c: &str) -> Option<(String, &str)> {
    if c.starts_with(['{', '[', '&', '*', '!', '?', '%', '@', '`']) { return None; }
    let (key, after) = match c.chars().next()? {
        q @ ('"' | '\'') => {
            let end = c[1..].find(q)? + 1;
            (c[1..end].to_string(), &c[end + 1..])
        }
        _ => {
            let i = c.find(": ").or(if c.ends_with(':') { Some(c.len() - 1) } else { None })?;
            (c[..i].to_string(), &c[i..])
        }
    };
    let rest = after.strip_prefix(':')?;
    if !(rest.is_empty() || rest.starts_with(' ')) || key == "<<" { return None; }
    Some((key, rest.trim()))
}

fn is_item(c: &str) -> bool { c == "-" || c.starts_with("- ") }

struct P { ls: Vec<(usize, String)>, i: usize }

impl P {
    /// The block that starts at the current line, whatever its indentation.
    fn block(&mut self) -> Option<Value> {
        let (ind, c) = self.ls.get(self.i)?;
        let ind = *ind;
        if is_item(c) { self.seq(ind) } else { self.map(ind) }
    }

    fn map(&mut self, ind: usize) -> Option<Value> {
        let mut m = Map::new();
        while let Some((i, c)) = self.ls.get(self.i).cloned() {
            if i < ind || (i == ind && is_item(&c)) { break; }
            if i > ind { return None; }
            let (k, rest) = split_key(&c)?;
            self.i += 1;
            let v = if rest.is_empty() {
                match self.ls.get(self.i) {
                    Some((j, _)) if *j > ind => self.block()?,
                    Some((j, d)) if *j == ind && is_item(d) => self.seq(ind)?,
                    _ => Value::Null,
                }
            } else if rest.starts_with(['|', '>']) {
                self.text(ind)
            } else {
                value(rest)?
            };
            if m.insert(k, v).is_some() { return None; }
        }
        Some(Value::Object(m))
    }

    fn seq(&mut self, ind: usize) -> Option<Value> {
        let mut a = vec![];
        while let Some((i, c)) = self.ls.get(self.i).cloned() {
            if i != ind || !is_item(&c) { break; }
            let rest = c[1..].trim_start();
            if rest.is_empty() {
                self.i += 1;
                match self.ls.get(self.i) {
                    Some((j, _)) if *j > ind => a.push(self.block()?),
                    _ => a.push(Value::Null),
                }
            } else if split_key(rest).is_some() || is_item(rest) {
                // `- key: value`: the rest of the line starts a nested block, indented to where the key begins
                self.ls[self.i] = (ind + c.len() - rest.len(), rest.to_string());
                a.push(self.block()?);
            } else {
                self.i += 1;
                a.push(value(rest)?);
            }
        }
        Some(Value::Array(a))
    }

    /// A `|` or `>` text: the deeper lines that follow, joined.
    fn text(&mut self, ind: usize) -> Value {
        let mut t: Vec<String> = vec![];
        while let Some((j, d)) = self.ls.get(self.i) {
            if *j <= ind { break; }
            t.push(d.clone());
            self.i += 1;
        }
        Value::String(t.join("\n"))
    }
}
