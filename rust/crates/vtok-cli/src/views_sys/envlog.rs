//! env / printenv (secrets masked) and log output (deduped, level counts, last errors).
use super::util::*;
use regex::Regex;
use std::collections::HashMap;

const SECRET: [&str; 10] = ["KEY", "SECRET", "TOKEN", "PASSWORD", "PASSWD", "PASS", "CREDENTIAL", "AUTH", "PRIVATE", "COOKIE"];

pub fn env(lines: &[String]) -> Option<String> {
    let re = Regex::new(r"^[A-Za-z_][A-Za-z0-9_]*=").ok()?;
    if lines.len() < 5 || lines.iter().filter(|l| re.is_match(l)).count() * 10 < lines.len() * 7 { return None; }
    let mut rows: Vec<String> = vec![];
    let mut masked = 0;
    for l in lines.iter().filter(|l| re.is_match(l)) {
        let (k, v) = l.split_once('=')?;
        let up = k.to_uppercase();
        let shown = if v.is_empty() { String::new() }
            else if SECRET.iter().any(|s| up.contains(s)) { masked += 1; "***".to_string() }
            else if k == "PATH" || (v.matches(':').count() >= 4 && v.starts_with('/')) { format!("<{} entries>", v.split(':').count()) }
            else { trunc(v, 70) };
        rows.push(format!("{k}={shown}"));
    }
    rows.sort();
    Some(format!("{} vars ({masked} secrets masked)\n{}\n", rows.len(), rows.join("\n")))
}

/// Log summary: level counts, repeated messages collapsed, last errors verbatim.
pub fn log(lines: &[String]) -> Option<String> {
    if lines.len() < 15 { return None; }
    let stamp = Regex::new(r"^\s*\[?\d{4}-\d\d-\d\d[T ]\d\d:\d\d:\d\d(?:[.,]\d+)?(?:Z|[+-]\d\d:?\d\d)?\]?\s*|^\s*[A-Z][a-z]{2}\s+\d+\s+\d\d:\d\d:\d\d\s+").ok()?;
    let digits = Regex::new(r"0x[0-9a-f]+|\b[0-9a-f]{8,}\b|\d+").ok()?;
    let level = Regex::new(r"\b(FATAL|CRITICAL|ERROR|ERR|WARN|WARNING|INFO|DEBUG|TRACE)\b").ok()?;
    let mut counts: Vec<(String, usize)> = vec![];
    let mut groups: HashMap<String, (usize, String)> = HashMap::new();
    let mut order: Vec<String> = vec![];
    let mut errors: Vec<String> = vec![];
    for l in lines.iter().filter(|l| !l.trim().is_empty()) {
        let lv = level.captures(l).map(|c| c[1].to_string()).unwrap_or_else(|| "OTHER".into());
        let lv = match lv.as_str() { "ERR" | "CRITICAL" => "ERROR".into(), "WARNING" => "WARN".into(), _ => lv };
        match counts.iter_mut().find(|(n, _)| *n == lv) { Some(c) => c.1 += 1, None => counts.push((lv.clone(), 1)) }
        if matches!(lv.as_str(), "ERROR" | "FATAL" | "WARN") {
            let body = stamp.replace(l, "").to_string();
            let key = digits.replace_all(&body, "#").to_string();
            let e = groups.entry(key.clone()).or_insert_with(|| { order.push(key.clone()); (0, body.clone()) });
            e.0 += 1;
            if lv != "WARN" { errors.push(trunc(l, 200)); }
        }
    }
    let mut o = format!("{} lines: {}\n", lines.len(), counts.iter().map(|(n, c)| format!("{n} {c}")).collect::<Vec<_>>().join(", "));
    for k in order.iter().take(12) {
        let (n, first) = &groups[k];
        o.push_str(&if *n > 1 { format!("{n}x {}\n", trunc(first, 160)) } else { format!("{}\n", trunc(first, 160)) });
    }
    if order.len() > 12 { o.push_str(&format!("... +{} more distinct messages\n", order.len() - 12)); }
    if let Some(last) = errors.last() { o.push_str(&format!("last error: {last}\n")); }
    o.push_str(&format!("tail:\n{}\n", lines.iter().rev().take(3).rev().map(|l| trunc(l, 160)).collect::<Vec<_>>().join("\n")));
    Some(o)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn l(s: &str) -> Vec<String> { s.lines().map(String::from).collect() }
    #[test]
    fn env_masks_secrets() {
        let raw = "HOME=/home/ana\nPATH=/usr/bin:/bin:/usr/local/bin:/opt/x:/opt/y\nGITHUB_TOKEN=ghp_abcdefghijklmnop\nAWS_SECRET_ACCESS_KEY=wJalr\nLANG=en_US.UTF-8";
        let o = env(&l(raw)).unwrap();
        assert!(o.contains("GITHUB_TOKEN=***") && o.contains("AWS_SECRET_ACCESS_KEY=***") && o.contains("PATH=<5 entries>"));
        assert!(!o.contains("ghp_") && !o.contains("wJalr") && o.contains("2 secrets masked") && o.contains("HOME=/home/ana"));
    }
    #[test]
    fn log_dedupes_and_counts() {
        let mut raw = String::new();
        for i in 0..40 { raw.push_str(&format!("2024-03-01 10:00:{:02} INFO request {i} served\n", i % 60)); }
        for i in 0..6 { raw.push_str(&format!("2024-03-01 10:01:{i:02} ERROR db timeout after {}ms\n", 100 + i)); }
        raw.push_str("2024-03-01 10:02:00 WARN disk at 91%\n2024-03-01 10:02:01 FATAL cannot continue\n");
        let lines = l(&raw);
        let o = log(&lines).unwrap();
        assert!(o.contains("INFO 40") && o.contains("ERROR 6") && o.contains("6x ERROR db timeout"));
        assert!(o.contains("last error: 2024-03-01 10:02:01 FATAL cannot continue"));
        assert!(!o.contains("request 7 served") && o.len() < raw.len());
    }
}
