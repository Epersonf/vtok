//! golangci-lint: one line per issue, counts per linter, source snippets and carets dropped.
use super::{cap, finish, uniq};
use regex::Regex;

pub fn view(lines: &[String]) -> Option<String> {
    let issue = Regex::new(r"^(\S+\.go:\d+(?::\d+)?:) (.*) \(([\w-]+)\)$").unwrap();
    let (mut issues, mut linters, mut extra) = (Vec::<String>::new(), Vec::<(String, usize)>::new(), Vec::<String>::new());
    for l in lines {
        if let Some(c) = issue.captures(l.trim_end()) {
            issues.push(format!("{} {} ({})", &c[1], &c[2], &c[3]));
            match linters.iter_mut().find(|x| x.0 == c[3]) {
                Some(x) => x.1 += 1,
                None => linters.push((c[3].to_string(), 1)),
            }
        } else if l.starts_with("level=") || l.starts_with("Error:") || l.ends_with(" issues.") || l.ends_with(" issue.") || l.starts_with("ERRO") {
            extra.push(l.trim().to_string());
        }
    }
    if issues.is_empty() && extra.is_empty() { return None; }
    uniq(&mut issues);
    let mut out = Vec::new();
    if !issues.is_empty() {
        let by: Vec<String> = linters.iter().map(|(n, c)| format!("{n} {c}")).collect();
        out.push(format!("{} issues: {}", issues.len(), by.join(", ")));
    }
    cap(&mut out, &issues, 50);
    uniq(&mut extra);
    out.extend(extra);
    finish(out, lines)
}
