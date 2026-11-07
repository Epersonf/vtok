//! golangci-lint JSON (`--out-format json`, `--output.json.path stdout`): issues grouped by linter, then file.
use super::{cap, finish};
use serde_json::Value;

/// Parse a JSON document that may be preceded by log lines (`level=...`).
fn doc(lines: &[String]) -> Option<Value> {
    let start = lines.iter().position(|l| l.trim_start().starts_with('{'))?;
    let text = lines[start..].join("\n");
    serde_json::Deserializer::from_str(&text).into_iter::<Value>().next()?.ok()
}

pub fn view(lines: &[String]) -> Option<String> {
    let v = doc(lines)?;
    let obj = v.as_object()?;
    if !obj.contains_key("Issues") { return None; }
    let issues = obj["Issues"].as_array().cloned().unwrap_or_default();
    // (linter, file, text) -> positions, in first-seen order
    let mut groups: Vec<(String, String, String, Vec<String>)> = Vec::new();
    for i in &issues {
        let (linter, text) = (i["FromLinter"].as_str().unwrap_or("?"), i["Text"].as_str().unwrap_or("").lines().next().unwrap_or(""));
        let (file, line, col) = (i["Pos"]["Filename"].as_str().unwrap_or("?"), i["Pos"]["Line"].as_u64().unwrap_or(0), i["Pos"]["Column"].as_u64().unwrap_or(0));
        let pos = if col > 0 { format!("{line}:{col}") } else { line.to_string() };
        match groups.iter_mut().find(|g| g.0 == linter && g.1 == file && g.2 == text) {
            Some(g) => g.3.push(pos),
            None => groups.push((linter.into(), file.into(), text.into(), vec![pos])),
        }
    }
    let mut out = Vec::new();
    if issues.is_empty() { out.push("golangci-lint: 0 issues".to_string()); return finish(out, lines); }
    let mut by: Vec<(String, usize)> = Vec::new();
    for g in &groups {
        match by.iter_mut().find(|b| b.0 == g.0) { Some(b) => b.1 += g.3.len(), None => by.push((g.0.clone(), g.3.len())) }
    }
    by.sort_by(|a, b| b.1.cmp(&a.1));
    out.push(format!("golangci-lint: {} issues: {}", issues.len(), by.iter().map(|(n, c)| format!("{n} {c}")).collect::<Vec<_>>().join(", ")));
    for (linter, n) in &by {
        out.push(format!("{linter} ({n})"));
        let rows: Vec<String> = groups.iter().filter(|g| &g.0 == linter).map(|g| format!("  {} {} {}", g.1, g.3.join(","), g.2)).collect();
        cap(&mut out, &rows, 25);
    }
    finish(out, lines)
}
