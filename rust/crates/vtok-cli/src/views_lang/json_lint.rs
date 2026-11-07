//! Linters' JSON reports: rubocop, eslint (`-f json`), sqlfluff, phpstan (`--error-format=json`).
use super::json_util::{doc, trim_common};
use super::util::{cap, finish, short, tally};
use serde_json::Value;

pub fn rubocop(lines: &[String]) -> Option<String> {
    let v = doc(lines)?;
    let files = v.get("files")?.as_array()?;
    let (mut rows, mut cops, mut fixable) = (Vec::new(), Vec::new(), 0);
    for f in files {
        for o in f["offenses"].as_array().into_iter().flatten() {
            let cop = o["cop_name"].as_str().unwrap_or("?");
            let sev = o["severity"].as_str().and_then(|s| s.chars().next()).unwrap_or('?').to_ascii_uppercase();
            if o["correctable"] == true { fixable += 1; }
            cops.push(cop.to_string());
            rows.push(format!("{}:{}:{} {sev} {cop}: {}", f["path"].as_str().unwrap_or(""), o["location"]["start_line"], o["location"]["start_column"], short(o["message"].as_str().unwrap_or(""), 140)));
        }
    }
    let mut out = cap(rows.clone(), 60);
    out.push(format!("rubocop: {} offenses in {} of {} files ({fixable} correctable)", rows.len(), files.iter().filter(|f| !f["offenses"].as_array().map_or(true, |o| o.is_empty())).count(), files.len()));
    if cops.len() > 8 { out.push(format!("by cop: {}", short(&tally(&cops), 300))); }
    finish(out)
}

pub fn eslint(lines: &[String]) -> Option<String> {
    let v = doc(lines)?;
    let files: Vec<&Value> = v.as_array()?.iter().filter(|f| f.get("filePath").is_some()).collect();
    if files.is_empty() { return None; }
    let paths: Vec<&str> = files.iter().map(|f| f["filePath"].as_str().unwrap_or("")).collect();
    let cut = trim_common(&paths);
    let (mut rows, mut rules, mut e, mut w) = (Vec::new(), Vec::new(), 0, 0);
    for (f, p) in files.iter().zip(&paths) {
        for m in f["messages"].as_array().into_iter().flatten() {
            let err = m["severity"] == 2;
            if err { e += 1 } else { w += 1 }
            let rule = m["ruleId"].as_str().unwrap_or("-");
            rules.push(rule.to_string());
            rows.push(format!("{}:{}:{} {} {rule} {}", &p[cut..], m["line"], m["column"], if err { "E" } else { "W" }, short(m["message"].as_str().unwrap_or(""), 140)));
        }
    }
    let mut out = cap(rows, 60);
    out.push(format!("eslint: {e} errors, {w} warnings in {} files", files.len()));
    if rules.len() > 8 { out.push(format!("by rule: {}", short(&tally(&rules), 300))); }
    finish(out)
}

pub fn sqlfluff(lines: &[String]) -> Option<String> {
    let v = doc(lines)?;
    let files: Vec<&Value> = v.as_array()?.iter().filter(|f| f.get("violations").is_some()).collect();
    if files.is_empty() { return None; }
    let (mut rows, mut codes) = (Vec::new(), Vec::new());
    for f in &files {
        for x in f["violations"].as_array().into_iter().flatten() {
            let code = x["code"].as_str().unwrap_or("?");
            codes.push(code.to_string());
            rows.push(format!("{}:{}:{} {code} {}", f["filepath"].as_str().unwrap_or(""), x["start_line_no"].as_u64().or(x["line_no"].as_u64()).unwrap_or(0), x["start_line_pos"].as_u64().or(x["line_pos"].as_u64()).unwrap_or(0), short(x["description"].as_str().unwrap_or(""), 140)));
        }
    }
    let mut out = cap(rows.clone(), 60);
    out.push(format!("sqlfluff: {} violations in {} files", rows.len(), files.iter().filter(|f| !f["violations"].as_array().map_or(true, |a| a.is_empty())).count()));
    if codes.len() > 8 { out.push(format!("by code: {}", short(&tally(&codes), 300))); }
    finish(out)
}

pub fn phpstan(lines: &[String]) -> Option<String> {
    let v = doc(lines)?;
    let files = v.get("files")?.as_object()?;
    let mut rows = Vec::new();
    for (path, f) in files {
        for m in f["messages"].as_array().into_iter().flatten() {
            let id = m["identifier"].as_str().map(|i| format!(" ({i})")).unwrap_or_default();
            rows.push(format!("{path}:{} {}{id}", m["line"], short(m["message"].as_str().unwrap_or(""), 160)));
        }
    }
    for e in v["errors"].as_array().into_iter().flatten() { rows.push(format!("error: {}", short(e.as_str().unwrap_or(""), 200))); }
    let mut out = cap(rows.clone(), 60);
    out.push(format!("phpstan: {} errors in {} files", v["totals"]["file_errors"].as_u64().unwrap_or(rows.len() as u64), files.len()));
    finish(out)
}
