//! eslint: stylish (the default), compact, unix and `-f json` reports. Every problem is kept, grouped by rule and message
//! (see `crate::diag`): the message once, then where it happens; variable names that differ share a template.
use super::json_util::doc;
use super::util::{finish, re};
use crate::diag::{render, D};
use serde_json::Value;

const NOUN: (&str, &str) = ("problem", "problems");

struct Parsed { ds: Vec<D>, headline: Option<String>, reported: Option<usize>, hint: Option<String>, tail: Vec<String> }

/// One `file` heading, then `  line:col  error|warning  message  rule` rows (stylish); or one self-contained row per
/// problem (compact, unix). The `✖ N problems (...)` line, the `--fix` hint and what follows them are kept as the summary.
fn parse(lines: &[String]) -> Parsed {
    let stylish = re(r"^\s+(\d+):(\d+)\s+(error|warning)\s+(.*?)(?:\s{2,}(\S+))?\s*$");
    let compact = re(r"^(.+?): line (\d+), col (\d+), (Error|Warning) - (.*?)(?: \(([^()\s]+)\))?\s*$");
    let unix = re(r"^(.+?):(\d+):(\d+): (.*?) \[(Error|Warning)(?:/([^\]]+))?\]\s*$");
    let headline = re(r"^(?:✖ )?(\d+) problems?\b");
    let hint = re(r"potentially fixable with the `--fix`");
    let mut p = Parsed { ds: vec![], headline: None, reported: None, hint: None, tail: vec![] };
    let mut file = String::new();
    for l in lines.iter().filter(|l| !l.trim().is_empty()) {
        let mut add = |f: &str, line: &str, col: &str, rule: &str, msg: &str, warn: bool| {
            let mut d = D::new(f, line.parse().unwrap_or(0), col.parse().unwrap_or(0), rule, msg.trim());
            d.warn = warn;
            p.ds.push(d);
        };
        if let Some(c) = stylish.captures(l) {
            add(&file, &c[1], &c[2], c.get(5).map_or("", |m| m.as_str()), &c[4], &c[3] == "warning");
        } else if let Some(c) = compact.captures(l) {
            add(&c[1], &c[2], &c[3], c.get(6).map_or("", |m| m.as_str()), &c[5], &c[4] == "Warning");
        } else if let Some(c) = unix.captures(l) {
            add(&c[1], &c[2], &c[3], c.get(6).map_or("", |m| m.as_str()), &c[4], &c[5] == "Warning");
        } else if let Some(c) = headline.captures(l) {
            p.reported = c[1].parse().ok();
            p.headline = Some(l.trim().to_string());
        } else if hint.is_match(l) {
            p.hint = Some(l.trim().to_string());
        } else if p.headline.is_some() {
            if p.tail.len() < 3 { p.tail.push(l.trim().to_string()); }
        } else if !l.starts_with(char::is_whitespace) {
            file = l.trim().to_string();
        }
    }
    p
}

/// The text formats. `min`: fewer problems than this and the output is left to the generic path.
pub fn text(lines: &[String], min: usize) -> Option<String> {
    let p = parse(lines);
    // the tool counted more problems than could be read: hand the raw text on rather than hide the rest
    if p.ds.is_empty() || p.ds.len() < min || p.reported.map_or(false, |n| n > p.ds.len()) { return None; }
    let summary: Vec<String> = p.headline.into_iter().chain(p.hint).chain(p.tail).collect();
    finish(render(&p.ds, NOUN, if summary.is_empty() { None } else { Some(summary.join("\n")) }))
}

/// `-f json`: an array with one object per file, each with `messages`.
pub fn json(lines: &[String]) -> Option<String> {
    let v = doc(lines)?;
    let files: Vec<&Value> = v.as_array()?.iter().filter(|f| f.get("filePath").is_some()).collect();
    if files.is_empty() { return None; }
    let (mut ds, mut errors, mut warnings) = (vec![], 0, 0);
    for f in &files {
        for m in f["messages"].as_array().into_iter().flatten() {
            let n = |k: &str| m[k].as_u64().unwrap_or(0) as u32;
            let mut d = D::new(f["filePath"].as_str().unwrap_or(""), n("line"), n("column"), m["ruleId"].as_str().unwrap_or(""), m["message"].as_str().unwrap_or("").trim());
            d.warn = m["severity"] != 2;
            if d.warn { warnings += 1 } else { errors += 1 }
            ds.push(d);
        }
    }
    let summary = format!("eslint: {errors} errors, {warnings} warnings in {} files", files.len());
    finish(render(&ds, NOUN, Some(summary)))
}
