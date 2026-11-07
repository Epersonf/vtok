//! gh, glab (GitHub/GitLab CLIs) and gt (Graphite): list, view and checks output.
use super::util::*;

fn is_date(s: &str) -> bool {
    let b = s.as_bytes();
    (b.len() >= 10 && b[4] == b'-' && b[7] == b'-' && b[..4].iter().all(u8::is_ascii_digit)) || s.ends_with(" ago") || s.starts_with("about ")
}

pub fn forge(args: &[String], lines: &[String]) -> Option<String> {
    let pos = positional(args);
    let sub = *pos.get(1)?;
    let area = *pos.first()?;
    let tabbed = lines.iter().filter(|l| l.contains('\t')).count() * 2 > lines.len();
    match (area, sub) {
        (_, "checks") if tabbed => checks(lines),
        ("run", "list") | ("pr", "list") | ("issue", "list") | ("mr", "list") | ("release", "list") | ("workflow", "list") if tabbed => tab_list(lines),
        (_, "view") | (_, "status") => view_text(lines),
        _ => None,
    }
}

fn tab_list(lines: &[String]) -> Option<String> {
    let mut o = String::new();
    for l in lines.iter().take(40) {
        let f: Vec<&str> = l.split('\t').map(str::trim).filter(|f| !f.is_empty() && !is_date(f)).collect();
        let (id, rest) = f.split_first()?;
        let id = if id.chars().all(|c| c.is_ascii_digit()) { format!("#{id}") } else { id.to_string() };
        o.push_str(&format!("{id} {}\n", rest.iter().map(|s| trunc(s, 70)).collect::<Vec<_>>().join(" | ")));
    }
    if lines.len() > 40 { o.push_str(&format!("... +{} more\n", lines.len() - 40)); }
    Some(o)
}

fn checks(lines: &[String]) -> Option<String> {
    let (mut pass, mut fail, mut pend, mut skip) = (0, vec![], vec![], 0);
    for l in lines {
        let f: Vec<&str> = l.split('\t').collect();
        let name = f.first()?.trim().to_string();
        match f.get(1).map(|s| s.trim()).unwrap_or("") {
            "pass" | "success" => pass += 1,
            "fail" | "failure" | "cancel" => fail.push(format!("{name} {}", f.get(3).unwrap_or(&"").trim())),
            "skipping" | "skipped" => skip += 1,
            _ => pend.push(name),
        }
    }
    let mut o = format!("checks: {pass} pass, {} fail, {} pending, {skip} skipped\n", fail.len(), pend.len());
    for f in &fail { o.push_str(&format!("FAIL {f}\n")); }
    if !pend.is_empty() { o.push_str(&format!("pending: {}\n", joined(&pend, 10))); }
    Some(o)
}

/// `gh pr view` style: `key:\tvalue` header, `--`, markdown body. Also tty-less run/issue views.
fn view_text(lines: &[String]) -> Option<String> {
    let mut o = String::new();
    let mut body = 0;
    let mut in_comment = false;
    for l in lines {
        let t = l.trim();
        if t.starts_with("<!--") { in_comment = !t.contains("-->"); continue; }
        if in_comment { in_comment = !t.contains("-->"); continue; }
        if t.is_empty() || t.starts_with("![") || t.starts_with("View this run on GitHub") || t.starts_with("To see what failed") { continue; }
        if t.starts_with("--") && t.chars().all(|c| c == '-') { continue; }
        if t.starts_with("number:") { continue; }
        let hdr = l.contains(":\t");
        if !hdr { body += 1; if body > 14 { continue; } }
        o.push_str(&format!("{}\n", trunc(&t.replace(":\t", ": "), 160)));
    }
    if body > 14 { o.push_str(&format!("... +{} body lines\n", body - 14)); }
    if o.is_empty() { None } else { Some(o) }
}

/// Graphite: drop hints, blank lines and pure graph connectors.
pub fn graphite(lines: &[String]) -> Option<String> {
    let keep: Vec<String> = lines.iter().map(|l| l.trim_end().to_string())
        .filter(|l| !l.trim().is_empty() && !l.trim().chars().all(|c| "│|┃ ".contains(c)) && !l.trim_start().to_lowercase().starts_with("tip:") && !l.contains("hint:"))
        .collect();
    if keep.is_empty() { return None; }
    Some(keep.join("\n") + "\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    fn l(s: &str) -> Vec<String> { s.lines().map(String::from).collect() }
    fn a(s: &str) -> Vec<String> { s.split_whitespace().map(String::from).collect() }
    #[test]
    fn pr_list_and_checks() {
        let raw = "123\tFix the parser bug\tfix/parser\tOPEN\t2024-03-01T10:00:00Z\n124\tAdd docs\tdocs\tDRAFT\t2024-03-02T10:00:00Z";
        let o = forge(&a("pr list"), &l(raw)).unwrap();
        assert!(o.contains("#123 Fix the parser bug | fix/parser | OPEN") && !o.contains("2024-03-01"));
        let c = l("build\tpass\t1m2s\thttps://x/1\nlint\tfail\t30s\thttps://x/2\ndeploy\tpending\t0\thttps://x/3\ndocs\tskipping\t0\thttps://x/4");
        let o = forge(&a("pr checks"), &c).unwrap();
        assert!(o.contains("1 pass, 1 fail, 1 pending, 1 skipped") && o.contains("FAIL lint https://x/2") && !o.contains("build"));
    }
    #[test]
    fn pr_view_trims_body() {
        let mut raw = String::from("title:\tAdd cache\nstate:\tOPEN\nauthor:\tana\nurl:\thttps://x/pull/9\n--\n<!-- template\nhidden -->\n## Summary\n![img](http://i.png)\n");
        for i in 0..30 { raw.push_str(&format!("body line {i}\n\n")); }
        let lines = l(&raw);
        let o = forge(&a("pr view 9"), &lines).unwrap();
        assert!(o.contains("title: Add cache") && o.contains("## Summary") && o.contains("+17 body lines"));
        assert!(!o.contains("hidden") && !o.contains("img") && o.len() < raw.len());
    }
    #[test]
    fn graphite_drops_hints() {
        let g = l("◉  feat-a (current)\n│\n◯  feat-b\n│\n\nTip: use gt sync\n");
        let o = graphite(&g).unwrap();
        assert!(o.contains("feat-a") && !o.contains("Tip") && o.lines().count() == 2);
    }
}
