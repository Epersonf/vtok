//! YAML scalars and one-line flow collections (`{cpu: 250m, ports: [80, 443]}`), read into JSON values.
use serde_json::{Map, Number, Value};

/// A scalar or a flow collection; None for anything this reader does not take on (anchors, tags, multi-line flows).
pub fn value(s: &str) -> Option<Value> {
    let s = s.trim();
    if s.starts_with('{') || s.starts_with('[') {
        let mut f = Flow { c: s.chars().collect(), i: 0 };
        let v = f.node(0)?;
        f.ws();
        return (f.i == f.c.len()).then_some(v);
    }
    plain(s)
}

fn plain(s: &str) -> Option<Value> {
    if s.starts_with(['"', '\'']) { return quoted(s).map(Value::String); }
    if s.starts_with(['&', '*', '!', '|', '>', '%', '@', '`']) { return None; }
    Some(match s {
        "" | "~" | "null" => Value::Null,
        "true" => Value::Bool(true),
        "false" => Value::Bool(false),
        _ => number(s).unwrap_or_else(|| Value::String(s.to_string())),
    })
}

/// Plain decimals only: `007`, `1e3` and `0x1F` stay text.
fn number(s: &str) -> Option<Value> {
    let t = s.strip_prefix('-').unwrap_or(s);
    let digits = !t.is_empty() && t.chars().all(|c| c.is_ascii_digit() || c == '.') && t.matches('.').count() <= 1;
    if !digits || t.starts_with('.') || t.ends_with('.') || (t.len() > 1 && t.starts_with('0') && !t.starts_with("0.")) { return None; }
    s.parse::<i64>().ok().map(Value::from).or_else(|| s.parse::<f64>().ok().and_then(Number::from_f64).map(Value::Number))
}

fn quoted(s: &str) -> Option<String> {
    let q = s.chars().next()?;
    if s.len() < 2 || !s.ends_with(q) { return None; }
    let inner = &s[1..s.len() - 1];
    if q == '\'' { return Some(inner.replace("''", "'")); }
    let (mut out, mut it) = (String::new(), inner.chars());
    while let Some(c) = it.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match it.next()? {
            'n' => out.push('\n'),
            't' => out.push('\t'),
            o @ ('"' | '\\' | '/') => out.push(o),
            o => {
                out.push('\\');
                out.push(o);
            }
        }
    }
    Some(out)
}

struct Flow { c: Vec<char>, i: usize }

impl Flow {
    fn ws(&mut self) { while self.c.get(self.i).map_or(false, |c| c.is_whitespace()) { self.i += 1; } }

    fn peek(&self) -> Option<char> { self.c.get(self.i).copied() }

    /// Text up to the next unquoted character of `stop`.
    fn token(&mut self, stop: &[char]) -> String {
        let start = self.i;
        let mut quote: Option<char> = None;
        while let Some(ch) = self.peek() {
            match quote {
                Some(q) if ch == q => quote = None,
                Some(_) => {}
                None if ch == '"' || ch == '\'' => quote = Some(ch),
                None if stop.contains(&ch) => break,
                None => {}
            }
            self.i += 1;
        }
        self.c[start..self.i].iter().collect::<String>().trim().to_string()
    }

    fn node(&mut self, depth: usize) -> Option<Value> {
        if depth > 12 { return None; }
        self.ws();
        match self.peek()? {
            '{' => {
                self.i += 1;
                let mut m = Map::new();
                loop {
                    self.ws();
                    if self.peek()? == '}' { self.i += 1; return Some(Value::Object(m)); }
                    let k = self.token(&[':']);
                    if self.peek()? != ':' { return None; }
                    self.i += 1;
                    let k = match plain(&k)? { Value::String(s) => s, other => other.to_string() };
                    m.insert(k, self.node(depth + 1)?);
                    self.sep('}')?;
                }
            }
            '[' => {
                self.i += 1;
                let mut a = vec![];
                loop {
                    self.ws();
                    if self.peek()? == ']' { self.i += 1; return Some(Value::Array(a)); }
                    a.push(self.node(depth + 1)?);
                    self.sep(']')?;
                }
            }
            _ => plain(&self.token(&[',', '}', ']'])),
        }
    }

    /// A comma, or the closing bracket (left in place).
    fn sep(&mut self, close: char) -> Option<()> {
        self.ws();
        match self.peek()? {
            ',' => { self.i += 1; Some(()) }
            c if c == close => Some(()),
            _ => None,
        }
    }
}
