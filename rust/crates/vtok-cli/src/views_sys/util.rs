//! Shared helpers for the sys views.

/// Split a command line into (tool, args): the last `&&`/`;` segment, env vars and wrappers skipped.
/// None when the output belongs to a pipeline stage we cannot see.
pub fn parse(cmd: &str) -> Option<(String, Vec<String>)> {
    let seg = cmd.rsplit(';').next()?.rsplit("&&").next()?.trim();
    if seg.contains('|') { return None; }
    let mut it = seg.split_whitespace().map(|s| s.trim_matches(|c| c == '"' || c == '\'').to_string()).peekable();
    while let Some(t) = it.peek() {
        let wrapper = matches!(t.as_str(), "sudo" | "command" | "exec" | "time" | "nice");
        if wrapper || (t.contains('=') && !t.starts_with('-') && !t.contains('/')) { it.next(); } else { break; }
    }
    let first = it.next()?;
    let base = first.rsplit(['/', '\\']).next()?.trim_end_matches(".exe").to_string();
    Some((base, it.collect()))
}

/// Non-flag arguments.
pub fn positional(args: &[String]) -> Vec<&str> {
    args.iter().map(|s| s.as_str()).filter(|a| !a.starts_with('-')).collect()
}

pub fn smaller(out: &str, lines: &[String]) -> bool {
    let n: usize = lines.iter().map(|l| l.len() + 1).sum();
    out.len() < n
}

pub fn human(bytes: u64) -> String {
    let (mut v, mut u) = (bytes as f64, 0);
    while v >= 1024.0 && u < 4 { v /= 1024.0; u += 1; }
    let s = ["B", "K", "M", "G", "T"][u];
    if u == 0 { format!("{bytes}B") } else if v >= 10.0 { format!("{v:.0}{s}") } else { format!("{v:.1}{s}") }
}

pub fn trunc(s: &str, n: usize) -> String {
    if s.chars().count() <= n { s.to_string() } else { format!("{}...", s.chars().take(n.saturating_sub(3)).collect::<String>()) }
}

/// Join items on one line, capped, with a "+N more" tail.
pub fn joined(items: &[String], cap: usize) -> String {
    let mut s = items.iter().take(cap).cloned().collect::<Vec<_>>().join(" ");
    if items.len() > cap { s.push_str(&format!(" +{} more", items.len() - cap)); }
    s
}

pub fn push_capped(out: &mut String, lines: &[String], cap: usize) {
    for l in lines.iter().take(cap) { out.push_str(l); out.push('\n'); }
    if lines.len() > cap { out.push_str(&format!("... +{} more\n", lines.len() - cap)); }
}

/// Parse a column table by the header's column starts (header columns separated by 2+ spaces).
pub fn table(lines: &[String]) -> Option<(Vec<String>, Vec<Vec<String>>)> {
    let head: Vec<char> = lines.first()?.chars().collect();
    let mut starts = vec![];
    for i in 0..head.len() {
        if head[i] != ' ' && (i == 0 || (i >= 2 && head[i - 1] == ' ' && head[i - 2] == ' ')) { starts.push(i); }
    }
    if starts.len() < 2 { return None; }
    let cut = |l: &str| -> Vec<String> {
        let c: Vec<char> = l.chars().collect();
        (0..starts.len()).map(|k| {
            let a = starts[k].min(c.len());
            let b = if k + 1 < starts.len() { starts[k + 1].min(c.len()) } else { c.len() };
            c[a..b].iter().collect::<String>().trim().to_string()
        }).collect()
    };
    let names = cut(&lines[0]);
    Some((names, lines[1..].iter().filter(|l| !l.trim().is_empty()).map(|l| cut(l)).collect()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_commands() {
        let (t, a) = parse("cd x && FOO=1 sudo git -C y status").unwrap();
        assert_eq!((t.as_str(), a.len()), ("git", 3));
        assert!(parse("git log | head").is_none());
    }
    #[test]
    fn humanizes() { assert_eq!(human(1536), "1.5K"); assert_eq!(human(12), "12B"); }
    #[test]
    fn table_cuts_columns() {
        let l = vec!["CONTAINER ID   IMAGE    NAMES".to_string(), "abc123         nginx    web".to_string()];
        let (h, r) = table(&l).unwrap();
        assert_eq!(h[0], "CONTAINER ID"); assert_eq!(r[0][2], "web");
    }
}
