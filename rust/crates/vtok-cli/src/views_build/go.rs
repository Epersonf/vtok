//! go test (text and -json), go build/vet/run: failures with messages and locations, package counts.
use super::{cap, finish, uniq};
use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;

pub fn view(sub: &str, args: &[String], lines: &[String]) -> Option<String> {
    match sub {
        "test" if args.iter().any(|a| a == "-json" || a == "--json") => test_json(lines),
        "test" => test_text(lines),
        "build" | "vet" | "install" | "run" | "generate" | "mod" | "get" => diag(sub, lines),
        _ => None,
    }
}

fn is_go_err(t: &str) -> bool {
    Regex::new(r"^[^\s:][^:]*\.go:\d+(:\d+)?: ").unwrap().is_match(t)
}

fn test_text(lines: &[String]) -> Option<String> {
    let (mut keep, mut ok, mut nofiles, mut failed) = (Vec::<String>::new(), 0, 0, 0);
    let (mut i, mut buf) = (0, Vec::<String>::new());
    while i < lines.len() {
        let t = lines[i].trim_end();
        i += 1;
        if t.starts_with("ok ") || t.starts_with("ok\t") {
            ok += 1;
        } else if t.starts_with('?') && t.contains("[no test files]") {
            nofiles += 1;
        } else if t.starts_with("=== RUN") || t.starts_with("--- PASS") {
            buf.clear();
        } else if (t.starts_with(' ') || t.starts_with('\t')) && !t.trim().is_empty() {
            buf.push(t.to_string());
        } else if t.starts_with("--- FAIL") {
            keep.append(&mut buf);
            keep.push(t.to_string());
            while i < lines.len() && (lines[i].starts_with(' ') || lines[i].starts_with('\t')) {
                keep.push(lines[i].trim_end().to_string());
                i += 1;
            }
        } else if t.starts_with("FAIL") && t.len() > 4 {
            failed += 1;
            keep.push(t.to_string());
        } else if t.starts_with("panic:") || t.starts_with("fatal error:") {
            keep.push(t.to_string());
            keep.extend(lines[i..].iter().map(|x| x.trim()).filter(|x| x.contains(".go:")).take(4).map(String::from));
        } else if t.starts_with("# ") || is_go_err(t) {
            keep.push(t.to_string());
        }
    }
    if ok + nofiles + failed + keep.len() == 0 { return None; }
    uniq(&mut keep);
    let mut out = Vec::new();
    cap(&mut out, &keep, 40);
    out.push(format!("go test: {ok} ok, {failed} FAIL, {nofiles} no test files"));
    finish(out, lines)
}

fn test_json(lines: &[String]) -> Option<String> {
    let mut outs: HashMap<(String, String), Vec<String>> = HashMap::new();
    let (mut tp, mut tf, mut ts, mut pp, mut pf, mut ps, mut parsed) = (0, 0, 0, 0, 0, 0, 0);
    let (mut failed_tests, mut failed_pkgs) = (Vec::<(String, String)>::new(), Vec::<String>::new());
    let mut stray = Vec::<String>::new();
    for l in lines {
        let Ok(v) = serde_json::from_str::<Value>(l) else {
            // build errors and toolchain chatter that go prints outside the event stream
            if !l.trim().is_empty() && !l.starts_with("go: ") { stray.push(l.trim_end().to_string()); }
            continue;
        };
        let Some(act) = v["Action"].as_str() else { continue };
        parsed += 1;
        let key = (v["Package"].as_str().unwrap_or("").to_string(), v["Test"].as_str().unwrap_or("").to_string());
        match (act, key.1.is_empty()) {
            ("output", _) => outs.entry(key).or_default().push(v["Output"].as_str().unwrap_or("").trim_end().to_string()),
            ("pass", false) => tp += 1,
            ("skip", false) => ts += 1,
            ("fail", false) => { tf += 1; failed_tests.push(key); }
            ("pass", true) => pp += 1,
            ("skip", true) => ps += 1,
            ("fail", true) => { pf += 1; failed_pkgs.push(key.0); }
            _ => {}
        }
    }
    if parsed == 0 { return None; }
    let noise = |s: &&String| {
        let t = s.trim();
        !(t.is_empty() || t.starts_with("=== ") || t.starts_with("--- ") || t == "PASS" || t == "FAIL" || t.starts_with("FAIL\t"))
    };
    let mut out = Vec::new();
    for k in &failed_tests {
        out.push(format!("FAIL {}.{}", k.0, k.1));
        let body: Vec<String> = outs.get(k).map(|o| o.iter().filter(noise).map(|s| format!("  {}", s.trim_start())).collect()).unwrap_or_default();
        cap(&mut out, &body, 10);
    }
    for pkg in failed_pkgs.iter().filter(|p| !failed_tests.iter().any(|k| &k.0 == *p)) {
        out.push(format!("FAIL {pkg}"));
        let body: Vec<String> = outs.get(&(pkg.clone(), String::new())).map(|o| o.iter().filter(noise).map(|s| format!("  {s}")).collect()).unwrap_or_default();
        cap(&mut out, &body, 15);
    }
    uniq(&mut stray);
    cap(&mut out, &stray, 20);
    out.push(format!("go test: packages {pp} ok, {pf} failed, {ps} no tests; tests {tp} passed, {tf} failed, {ts} skipped"));
    finish(out, lines)
}

fn diag(sub: &str, lines: &[String]) -> Option<String> {
    let mut kept: Vec<String> = Vec::new();
    let mut downloads = 0;
    for l in lines {
        let t = l.trim_end();
        if t.starts_with("go: downloading") || t.starts_with("go: finding") || t.starts_with("go: extracting") { downloads += 1; }
        else if !t.trim().is_empty() { kept.push(t.to_string()); }
    }
    uniq(&mut kept);
    let mut out = Vec::new();
    cap(&mut out, &kept, 40);
    if kept.is_empty() { out.push(format!("go {sub}: ok ({downloads} modules downloaded)")); }
    finish(out, lines)
}
