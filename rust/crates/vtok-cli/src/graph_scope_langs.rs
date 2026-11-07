//! Import readers for Python, TypeScript / JavaScript (ES imports, require), Go and Java (package = directory) and
//! the C family (`#include "x.h"`), filling a file's `Scope`.
use super::edges::Parsed;
use super::scope::{dir_of, normalize, Files, Scope};
use regex::Regex;
use std::sync::OnceLock;

fn re(cell: &'static OnceLock<Regex>, pat: &str) -> &'static Regex { cell.get_or_init(|| Regex::new(pat).unwrap()) }

fn bind(s: &mut Scope, local: &str, orig: &str, files: Vec<usize>) { s.names.insert(local.to_string(), (orig.to_string(), files)); }

pub(crate) fn python(i: usize, parsed: &[Parsed], fs: &Files, s: &mut Scope) {
    static FROM: OnceLock<Regex> = OnceLock::new();
    static IMP: OnceLock<Regex> = OnceLock::new();
    let p = &parsed[i];
    let resolve = |m: &str| -> Vec<usize> {
        let dots = m.len() - m.trim_start_matches('.').len();
        let rest = m.trim_start_matches('.').replace('.', "/");
        if dots > 0 {
            let mut d = dir_of(&p.rel).to_string();
            for _ in 1..dots { d = dir_of(&d).to_string(); }
            let base = normalize(&format!("{d}/{rest}"));
            return [".py", "/__init__.py"].iter().filter_map(|e| fs.by_path.get(&format!("{base}{e}")).copied()).collect();
        }
        let mut v = fs.module(parsed, &rest, &[".py", "/__init__.py"]);
        v.sort_by_key(|&j| std::cmp::Reverse(common_prefix(&parsed[j].rel, &p.rel)));
        v.truncate(1);
        v
    };
    for c in re(&FROM, r"(?m)^\s*from\s+([.\w]+)\s+import\s+\(?([^)\n]+)").captures_iter(&p.src) {
        let files = resolve(&c[1]);
        for part in c[2].split(',') {
            let mut w = part.split_whitespace();
            let Some(orig) = w.next() else { continue };
            if orig == "*" { s.globs.extend(&files); continue; }
            let local = if w.next() == Some("as") { w.next().unwrap_or(orig) } else { orig };
            let sub = resolve(&format!("{}{}{orig}", &c[1], if c[1].ends_with('.') { "" } else { "." }));
            if !sub.is_empty() { s.mods.insert(local.to_string(), sub); } else { bind(s, local, orig, files.clone()); }
        }
    }
    for c in re(&IMP, r"(?m)^\s*import\s+([\w.]+)(?:\s+as\s+(\w+))?").captures_iter(&p.src) {
        let files = resolve(&c[1]);
        let local = c.get(2).map_or_else(|| c[1].rsplit('.').next().unwrap_or(&c[1]).to_string(), |m| m.as_str().to_string());
        s.mods.insert(local, files);
    }
}

fn common_prefix(a: &str, b: &str) -> usize { a.chars().zip(b.chars()).take_while(|(x, y)| x == y).count() }

pub(crate) fn typescript(i: usize, parsed: &[Parsed], fs: &Files, s: &mut Scope) {
    static IMP: OnceLock<Regex> = OnceLock::new();
    static REQ: OnceLock<Regex> = OnceLock::new();
    let p = &parsed[i];
    let exts = [".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs", ".mts", ".cts", "/index.ts", "/index.tsx", "/index.js"];
    let resolve = |m: &str| -> Vec<usize> {
        if m.starts_with('.') {
            let base = normalize(&format!("{}/{m}", dir_of(&p.rel)));
            if let Some(&j) = fs.by_path.get(&base) { return vec![j]; }
            return exts.iter().filter_map(|e| fs.by_path.get(&format!("{base}{e}")).copied()).take(1).collect();
        }
        let m = m.trim_start_matches("@/").trim_start_matches("~/");
        if m.contains('/') && !m.starts_with('@') { fs.module(parsed, m, &exts).into_iter().take(1).collect() } else { vec![] }
    };
    let mut any = false;
    for c in re(&IMP, r#"(?s)\b(import|export)\s+(type\s+)?([^;'"]*?)\s*from\s*['"]([^'"]+)['"]"#).captures_iter(&p.src) {
        any = true;
        let files = resolve(&c[4]);
        let what = c[3].trim();
        if c[1].starts_with('e') { s.reexports.extend(&files); }
        if what == "*" { s.globs.extend(&files); continue; }
        if let Some(ns) = what.strip_prefix("* as ") { s.mods.insert(ns.trim().to_string(), files.clone()); continue; }
        let (default, named) = match what.find('{') { Some(k) => (what[..k].trim().trim_end_matches(','), &what[k..]), None => (what, "") };
        if !default.is_empty() { bind(s, default.trim(), default.trim(), files.clone()); s.mods.insert(default.trim().to_string(), files.clone()); }
        for part in named.trim_matches(|c| c == '{' || c == '}').split(',') {
            let mut w = part.split_whitespace().filter(|w| *w != "type");
            let Some(orig) = w.next() else { continue };
            let local = if w.next() == Some("as") { w.next().unwrap_or(orig) } else { orig };
            bind(s, local, orig, files.clone());
        }
    }
    for c in re(&REQ, r#"\b(?:const|let|var)\s+(\{[^}]*\}|\w+)\s*=\s*require\(\s*['"]([^'"]+)['"]"#).captures_iter(&p.src) {
        any = true;
        let files = resolve(&c[2]);
        if c[1].starts_with('{') { for w in c[1].trim_matches(|c| c == '{' || c == '}').split(',') { let n = w.split(':').last().unwrap_or(w).trim(); if !n.is_empty() { bind(s, n, w.split(':').next().unwrap_or(n).trim(), files.clone()); } } }
        else { s.mods.insert(c[1].to_string(), files); }
    }
    s.strict = any;
}

pub(crate) fn package_dir(i: usize, parsed: &[Parsed], fs: &Files, s: &mut Scope, lang: &str) {
    static GO: OnceLock<Regex> = OnceLock::new();
    static JAVA: OnceLock<Regex> = OnceLock::new();
    static INC: OnceLock<Regex> = OnceLock::new();
    let p = &parsed[i];
    if lang == "go" || lang == "java" {
        s.strict = true;
        s.globs.extend(fs.by_dir.get(dir_of(&p.rel)).into_iter().flatten().copied().filter(|&j| j != i && parsed[j].lang == p.lang));
    }
    if lang == "go" {
        for c in re(&GO, r#"(?m)^\s*(?:import\s+)?(\w+\s+)?"([\w./-]+)"\s*$"#).captures_iter(&p.src) {
            let d = &c[2];
            let local = c.get(1).map_or_else(|| d.rsplit('/').next().unwrap_or(d).to_string(), |m| m.as_str().trim().to_string());
            s.mods.insert(local, fs.dir_suffix(d.rsplit('/').next().unwrap_or(d)).into_iter().filter(|&j| parsed[j].rel.ends_with(".go")).collect());
        }
    } else if lang == "java" {
        for c in re(&JAVA, r"(?m)^\s*import\s+(?:static\s+)?([\w.]+)\s*;").captures_iter(&p.src) {
            let path = c[1].replace('.', "/");
            if let Some(d) = path.strip_suffix("/*") { s.globs.extend(fs.dir_suffix(d)); continue; }
            let files = fs.module(parsed, &path, &[".java"]);
            let n = path.rsplit('/').next().unwrap_or(&path).to_string();
            s.mods.insert(n.clone(), files.clone());
            bind(s, &n, &n, files);
        }
    } else {
        for c in re(&INC, r#"(?m)^\s*#\s*include\s*"([^"]+)""#).captures_iter(&p.src) {
            let base = c[1].rsplit_once('.').map_or(&c[1][..], |x| x.0).to_string();
            s.globs.extend(fs.module(parsed, &base, &[".h", ".hpp", ".hh", ".c", ".cc", ".cpp"]));
        }
    }
}
