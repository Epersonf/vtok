//! sbt: [error] lines, ScalaTest/munit failures with their message, test totals, [success] line.
//! Drops [info] progress, passing test names, suite headers and warnings (counted).
use super::{finish, trunc, uniq};
use regex::Regex;

pub fn view(lines: &[String]) -> Option<String> {
    let pre = Regex::new(r"^\[(info|warn|error|success|debug)\] ?(.*)$").unwrap();
    let keep = ["Tests:", "Passed:", "Failed:", "All tests passed", "Total number of tests", "Suites:", "Run completed", "Failed tests", "*** ", "Total time"];
    let (mut out, mut warns, mut known) = (Vec::<String>::new(), 0, false);
    let mut i = 0;
    while i < lines.len() {
        let Some(c) = pre.captures(lines[i].trim_end()) else { i += 1; continue };
        i += 1;
        known = true;
        let body = c[2].trim_end();
        match &c[1] {
            "error" if !body.is_empty() => out.push(trunc(&format!("[error] {body}"), 300)),
            "warn" => warns += 1,
            "success" => out.push(format!("[success] {body}")),
            "info" if body.contains("*** FAILED ***") || body.contains("*** ABORTED ***") || body.contains("==> X ") => {
                out.push(body.trim().to_string());
                while let Some(n) = lines.get(i).and_then(|x| pre.captures(x)) {
                    let nb = n[2].to_string();
                    if !nb.starts_with("  ") || nb.trim_start().starts_with("at ") || out.len() > 200 { break; }
                    out.push(format!("  {}", trunc(nb.trim(), 250)));
                    i += 1;
                }
            }
            "info" if keep.iter().any(|k| body.starts_with(k)) => out.push(body.to_string()),
            _ => {}
        }
    }
    if !known { return None; }
    uniq(&mut out);
    if warns > 0 { out.insert(0, format!("({warns} warnings dropped)")); }
    finish(out, lines)
}
