//! Test runners' JSON reporters: playwright (`--reporter=json`) and rspec (`--format json`).
use super::json_util::{doc, ansi, first_lines};
use super::util::{cap, finish};
use serde_json::Value;

pub fn playwright(lines: &[String]) -> Option<String> {
    let v = doc(lines)?;
    let st = v.get("stats")?;
    let n = |k: &str| st[k].as_u64().unwrap_or(0);
    let mut out = vec![format!("playwright: {} passed, {} failed, {} flaky, {} skipped ({:.1}s)", n("expected"), n("unexpected"), n("flaky"), n("skipped"), st["duration"].as_f64().unwrap_or(0.0) / 1000.0)];
    let mut fails = Vec::new();
    for s in v["suites"].as_array().into_iter().flatten() { walk(s, &mut Vec::new(), &mut fails); }
    for e in v["errors"].as_array().into_iter().flatten() {
        fails.push(format!("ERROR {}", first_lines(&ansi(e["message"].as_str().unwrap_or("")), 3, 200)));
    }
    out.extend(cap(fails, 30));
    finish(out)
}

fn walk(s: &Value, path: &mut Vec<String>, fails: &mut Vec<String>) {
    let title = s["title"].as_str().unwrap_or("").to_string();
    let named = !title.is_empty() && s["file"].as_str() != Some(title.as_str()) && !title.ends_with(".spec.ts") && !title.ends_with(".test.ts") && !title.ends_with(".js");
    if named { path.push(title.clone()); }
    for sp in s["specs"].as_array().into_iter().flatten() {
        for t in sp["tests"].as_array().into_iter().flatten() {
            let status = t["status"].as_str().unwrap_or("");
            if status == "expected" || status == "skipped" { continue; }
            let file = sp["file"].as_str().map(String::from).unwrap_or_else(|| if named { String::new() } else { title.clone() });
            let loc = if sp["line"].is_u64() { format!("{file}:{}", sp["line"]) } else { file };
            let proj = t["projectName"].as_str().map(|p| format!(" [{p}]")).unwrap_or_default();
            let label = if status == "flaky" { "FLAKY" } else { "FAIL" };
            fails.push(format!("{label} {loc} > {}{proj}", [path.clone(), vec![sp["title"].as_str().unwrap_or("").into()]].concat().join(" > ")));
            if let Some(r) = t["results"].as_array().and_then(|r| r.iter().rev().find(|r| r["errors"].as_array().map_or(false, |e| !e.is_empty()))) {
                let m = r["errors"][0]["message"].as_str().unwrap_or("");
                fails.push(format!("  {}", first_lines(&ansi(m), 4, 200).replace('\n', "\n  ")));
            }
        }
    }
    for c in s["suites"].as_array().into_iter().flatten() { walk(c, path, fails); }
    if named { path.pop(); }
}

pub fn rspec(lines: &[String]) -> Option<String> {
    let v = doc(lines)?;
    let sm = v.get("summary")?;
    let ex = v["examples"].as_array()?;
    let mut out = vec![v["summary_line"].as_str().map(String::from).unwrap_or_else(|| format!("{} examples, {} failures", ex.len(), sm["failure_count"]))];
    if let Some(d) = sm["duration"].as_f64() { out[0] += &format!(" ({d:.1}s)"); }
    let mut rows = Vec::new();
    for e in ex.iter().filter(|e| e["status"] == "failed") {
        rows.push(format!("FAIL {}:{} {}", e["file_path"].as_str().unwrap_or("").trim_start_matches("./"), e["line_number"], e["full_description"].as_str().unwrap_or("")));
        let x = &e["exception"];
        rows.push(format!("  {}: {}", x["class"].as_str().unwrap_or("Error"), first_lines(x["message"].as_str().unwrap_or(""), 4, 200).replace('\n', "\n    ")));
        if let Some(b) = x["backtrace"].as_array().and_then(|b| b.iter().filter_map(|l| l.as_str()).find(|l| !l.contains("/gems/") && !l.contains("/lib/ruby/"))) { rows.push(format!("  at {b}")); }
    }
    out.extend(cap(rows, 40));
    let pend: Vec<String> = ex.iter().filter(|e| e["status"] == "pending").map(|e| format!("PENDING {}:{} {}", e["file_path"].as_str().unwrap_or("").trim_start_matches("./"), e["line_number"], e["full_description"].as_str().unwrap_or(""))).collect();
    out.extend(cap(pend, 10));
    if let Some(e) = sm["errors_outside_of_examples_count"].as_u64().filter(|n| *n > 0) { out.push(format!("{e} errors outside of examples")); }
    finish(out)
}
