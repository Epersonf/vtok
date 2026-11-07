//! A tiny grammar-independent lexer for strings and comments. When a file has regions the grammar cannot parse, its
//! error recovery may read a string as code (a C string continued with `\` at line ends, say) and the minifier would
//! then change the string's whitespace or drop "comments" inside it. The lexer's string spans are therefore kept
//! verbatim by the minifier, and the roundtrip proof also compares the files lexically: same string literals, same
//! code outside strings and comments (whitespace aside).
use crate::spec::{Mode, Spec};

#[derive(Clone, Copy, PartialEq)]
enum Fam { C, Sql, Css, Html }

fn fam(spec: &Spec) -> Fam {
    match spec.mode { Mode::Html => Fam::Html, Mode::Css => Fam::Css, Mode::Code if spec.stmts => Fam::Sql, Mode::Code => Fam::C }
}

fn find(b: &[u8], from: usize, pat: &[u8]) -> usize { (from..b.len()).find(|&j| b[j..].starts_with(pat)).map_or(b.len(), |j| j + pat.len()) }

/// End of a quoted literal opened at `i` (quote `q`), with backslash escapes unless `raw`; doubled quotes when `dbl`.
fn quoted(b: &[u8], i: usize, q: u8, raw: bool, dbl: bool) -> usize {
    let mut j = i + 1;
    while j < b.len() {
        if b[j] == b'\\' && !raw { j += 2; continue; }
        if b[j] == q { if dbl && b.get(j + 1) == Some(&q) { j += 2; continue; } return j + 1; }
        j += 1;
    }
    b.len()
}

/// Byte spans of string literals (`true`) and comments (`false`), in order.
pub fn regions(spec: &Spec, src: &str) -> Vec<(usize, usize, bool)> {
    let (f, b) = (fam(spec), src.as_bytes());
    let (mut out, mut i) = (vec![], 0);
    while i < b.len() {
        let c = b[i];
        let at = |p: &[u8]| b[i..].starts_with(p);
        let (end, string) = match f {
            Fam::Html if at(b"<!--") => (find(b, i + 4, b"-->"), false),
            Fam::Html => { i += 1; continue; }
            _ if at(b"/*") => (find(b, i + 2, b"*/"), false),
            Fam::Sql if at(b"--") => (find(b, i, b"\n"), false),
            Fam::C if at(b"//") => (find(b, i, b"\n"), false),
            Fam::Sql if c == b'$' && (i == 0 || !(b[i - 1].is_ascii_alphanumeric() || b[i - 1] == b'_')) => {
                let tag = (i + 1..b.len()).find(|&j| !(b[j].is_ascii_alphanumeric() || b[j] == b'_')).filter(|&j| b[j] == b'$');
                match tag { Some(j) => (find(b, j + 1, &b[i..=j]), true), None => { i += 1; continue; } }
            }
            Fam::Sql if c == b'\'' || c == b'"' => (quoted(b, i, c, true, true), true),
            Fam::C if at(b"'''") || at(b"\"\"\"") => (find(b, i + 3, &b[i..i + 3]), true),
            Fam::C if at(b"@\"") => (quoted(b, i + 1, b'"', true, true), true),
            Fam::C if c == b'`' => (quoted(b, i, c, true, false), true),
            Fam::C | Fam::Css if c == b'"' || c == b'\'' => (quoted(b, i, c, false, false), true),
            _ => { i += 1; continue; }
        };
        out.push((i, end.min(b.len()), string));
        i = end.max(i + 1);
    }
    out
}

/// What the lexical proof compares: the string literals, and the rest without comments and whitespace. A SQL
/// dollar-quoted body is code to the grammar (it parses and minifies function bodies), so it is read as code too.
pub fn proof(spec: &Spec, src: &str) -> (Vec<String>, String) {
    let (mut strs, mut code) = (vec![], String::new());
    proof_into(spec, src, &mut strs, &mut code);
    (strs, code)
}

fn proof_into(spec: &Spec, src: &str, strs: &mut Vec<String>, code: &mut String) {
    let squash = |s: &str, code: &mut String| code.extend(s.replace("\\\n", "").chars().filter(|c| !c.is_whitespace() && *c != '\u{feff}'));
    let mut at = 0;
    for (a, e, string) in regions(spec, src) {
        squash(&src[at..a], code);
        let lit = &src[a..e];
        let tag = lit[1..].find('$').map(|j| &lit[..j + 2]).filter(|t| fam(spec) == Fam::Sql && lit.starts_with('$') && lit.len() >= 2 * t.len() && lit.ends_with(*t));
        match tag {
            Some(t) => { code.push_str(t); proof_into(spec, &lit[t.len()..lit.len() - t.len()], strs, code); code.push_str(t); }
            None if string => { strs.push(lit.to_string()); code.push('\u{1}'); }
            None => {}
        }
        at = e;
    }
    squash(&src[at.min(src.len())..], code);
}

/// Spans the minifier must copy exactly (string literals by the lexer's reading).
pub fn protected(spec: &Spec, src: &str) -> Vec<(usize, usize)> {
    regions(spec, src).into_iter().filter(|r| r.2).map(|r| (r.0, r.1)).collect()
}
