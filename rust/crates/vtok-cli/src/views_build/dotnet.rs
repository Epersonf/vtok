//! dotnet build/test/restore/format/publish and msbuild: errors with file(line,col) and code, warnings grouped
//! by code, failing tests with message and first stack frame, test totals, "Build succeeded/FAILED", elapsed.
use super::{finish, trunc, uniq, Groups};
use regex::Regex;
use std::collections::HashSet;

pub fn view(sub: &str, lines: &[String]) -> Option<String> {
    if let Some(o) = super::dotnet_trx::view(lines) { return Some(o); }
    if !matches!(sub, "build" | "test" | "restore" | "format" | "publish" | "pack" | "msbuild" | "clean" | "run") { return None; }
    let diag = Regex::new(r"^\s*(?:(.+?)(?:\((\d+)(?:,(\d+))?\))?\s*: )?(error|warning) ([A-Za-z][A-Za-z0-9_]*): (.*?)(?: \[[^\]]+\])?$").unwrap();
    let total = Regex::new(r"^(Passed|Failed)!\s+-\s+").unwrap();
    let vs = Regex::new(r"^\s*(Total tests|Passed|Failed|Skipped): \d+|^Test [Rr]un (Successful|Failed)").unwrap();
    let (mut errors, mut warns, mut out) = (Vec::<String>::new(), Groups::default(), Vec::<String>::new());
    let (mut restored, mut built, mut formatted, mut known) = (0, 0, 0, false);
    let mut seen = HashSet::new();
    let mut i = 0;
    while i < lines.len() {
        let l = lines[i].trim_end();
        let t = l.trim_start();
        i += 1;
        if let Some(c) = diag.captures(l) {
            if !seen.insert(t.to_string()) { continue; }
            known = true;
            let file = c.get(1).map_or("", |m| m.as_str());
            let at = match (c.get(2), c.get(3)) {
                (Some(a), Some(b)) => format!("{file}:{}:{}", a.as_str(), b.as_str()),
                (Some(a), None) => format!("{file}:{}", a.as_str()),
                _ => file.to_string(),
            };
            if &c[4] == "error" { errors.push(format!("{at} error {}: {}", &c[5], trunc(&c[6], 250))); }
            else { warns.add(&format!("warning {}", &c[5]), &format!("{at} {}", trunc(&c[6], 120))); }
        } else if total.is_match(t) || vs.is_match(l) || t.starts_with("Build succeeded") || t.starts_with("Build FAILED") || t.starts_with("Time Elapsed") || t.starts_with("Restore complete") || t.starts_with("Build succeeded in") {
            known = true;
            out.push(t.to_string());
        } else if t.ends_with("Error(s)") || (t.ends_with("Warning(s)") && !t.starts_with('0')) {
            out.push(t.to_string());
        } else if t.starts_with("Restored ") { known = true; restored += 1; }
        else if t.contains(" -> ") && (t.ends_with(".dll") || t.ends_with(".exe") || t.ends_with('/') || t.ends_with('\\')) { known = true; built += 1; }
        else if t.starts_with("Formatted code file") { known = true; formatted += 1; }
        else if t.starts_with("Determining projects") || t.starts_with("Starting test execution") { known = true; }
        else if t.starts_with("Failed ") && t.ends_with(']') {
            known = true;
            out.push(format!("FAILED {}", t[7..].rsplit_once(" [").map_or(&t[7..], |x| x.0)));
            let mut in_msg = false;
            while i < lines.len() {
                let n = lines[i].trim();
                if n.starts_with("Passed") || n.starts_with("Failed!") || total.is_match(n) || n.starts_with("Skipped ") || (n.starts_with("Failed ") && n.ends_with(']')) { break; }
                if n.starts_with("Stack Trace:") {
                    in_msg = false;
                    let frames: Vec<&str> = lines[i + 1..].iter().map(|x| x.trim()).take_while(|x| x.starts_with("at ")).collect();
                    if let Some(f) = frames.iter().find(|f| f.contains(" in ")).or(frames.first()) { out.push(format!("    {}", trunc(f, 250))); }
                } else if n.starts_with("Error Message:") { in_msg = true; }
                else if in_msg && !n.is_empty() { out.push(format!("    {}", trunc(n, 250))); }
                i += 1;
            }
        }
    }
    if !known { return None; }
    uniq(&mut errors);
    let mut res = errors;
    warns.render(&mut res);
    res.extend(out);
    if restored + built + formatted > 0 { res.push(format!("({restored} restored, {built} built, {formatted} formatted)")); }
    finish(res, lines)
}
