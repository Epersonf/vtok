//! `git diff --stat` / `--name-status` (and `git show --stat --format=`): already compact, so they pass through;
//! only a listing bigger than the budget is cut, keeping the summary line and saying how much was left out.
use vtok_api::ntok;

fn is_stat(l: &str) -> bool {
    let t = l.trim_start();
    (l.starts_with(' ') && t.contains(" | ") && t.rsplit(" | ").next().map_or(false, |r| {
        let r = r.trim();
        r.starts_with("Bin") || r.split_whitespace().next().map_or(false, |n| n.bytes().all(|b| b.is_ascii_digit()))
    })) || is_summary(t)
}

fn is_summary(t: &str) -> bool { t.contains(" changed") && t.contains("file") && (t.contains("insertion") || t.contains("deletion") || t.ends_with("changed")) }

fn is_name_status(l: &str) -> bool {
    let mut it = l.splitn(2, '\t');
    let code = it.next().unwrap_or("");
    it.next().is_some() && !code.is_empty() && code.len() <= 4
        && code.chars().next().map_or(false, |c| "ACDMRTUXB".contains(c)) && code[1..].bytes().all(|b| b.is_ascii_digit())
}

pub fn view(lines: &[String], budget: usize) -> Option<String> {
    let rows: Vec<&String> = lines.iter().filter(|l| !l.trim().is_empty()).collect();
    if rows.len() < 2 { return None; }
    let stat = rows.iter().all(|l| is_stat(l));
    if !stat && !rows.iter().all(|l| is_name_status(l)) { return None; }
    let raw = ntok(&lines.join("\n"));
    let b = super::budget_for(raw, budget);
    if raw <= b { return Some(lines.join("\n") + "\n"); }
    let mut out = String::new();
    let mut used = 0;
    let mut shown = 0;
    let body: Vec<&&String> = rows.iter().filter(|l| !is_summary(l.trim_start())).collect();
    for l in &body {
        used += ntok(l) + 1;
        if used > b { break; }
        out.push_str(l);
        out.push('\n');
        shown += 1;
    }
    out.push_str(&format!("… {} more files (vtok output ID --grep PATTERN)\n", body.len() - shown));
    if let Some(s) = rows.iter().find(|l| is_summary(l.trim_start())) { out.push_str(s); out.push('\n'); }
    Some(out)
}
