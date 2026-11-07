//! aws cli, curl and wget.
use super::util::*;
use serde_json::Value;

pub fn aws(args: &[String], lines: &[String]) -> Option<String> {
    let text = lines.join("\n");
    if let Ok(v) = serde_json::from_str::<Value>(text.trim()) {
        return Some(super::jsonv::summarize(&v, true, 60));
    }
    let pos = positional(args);
    if pos.first() == Some(&"s3") && pos.get(1) == Some(&"ls") {
        let (mut dirs, mut files, mut total) = (vec![], vec![], 0u64);
        for l in lines {
            let t = l.trim();
            if let Some(d) = t.strip_prefix("PRE ") { dirs.push(d.to_string()); continue; }
            let rest: Vec<&str> = t.split_whitespace().collect();
            if rest.len() >= 4 {
                let size: u64 = rest[2].parse().unwrap_or(0);
                total += size;
                files.push(format!("{} {}", rest[3..].join(" "), human(size)));
            }
        }
        let mut o = format!("{} dirs, {} files ({})\n", dirs.len(), files.len(), human(total));
        if !dirs.is_empty() { o.push_str(&format!("dirs: {}\n", joined(&dirs, 30))); }
        if !files.is_empty() { o.push_str(&format!("files: {}\n", joined(&files, 30))); }
        return Some(o);
    }
    None
}

fn is_meter(l: &str) -> bool {
    let t = l.trim_start();
    if t.starts_with("% Total") || t.contains("Dload  Upload") || t.starts_with("} [") || t.starts_with("{ [") { return true; }
    regex::Regex::new(r"^\d+\s+\S+\s+\d+\s+\S+\s+\d+\s+\d+\s+\S+\s+\d+\s+[-\d:]+").map_or(false, |r| r.is_match(t))
}

const KEEP_HDR: [&str; 9] = ["content-type", "content-length", "location", "server", "cache-control", "www-authenticate", "retry-after", "x-ratelimit-remaining", "etag"];

fn hdr(h: &str, o: &mut String, cookies: &mut usize) {
    let low = h.to_lowercase();
    if low.starts_with("set-cookie") { *cookies += 1; }
    else if KEEP_HDR.iter().any(|k| low.starts_with(k)) { o.push_str(&format!("{}\n", h.trim())); }
}

/// curl output: progress meter dropped, status + key headers, body summarised (JSON by structure with short values).
pub fn curl(lines: &[String]) -> Option<String> {
    let mut o = String::new();
    let (mut body, mut cookies, mut in_hdr) = (vec![], 0, false);
    for l in lines {
        if is_meter(l) || l.starts_with("* ") { continue; }
        if l == "<" { in_hdr = false; continue; }
        if let Some(r) = l.strip_prefix("> ") {
            if r.contains(" HTTP/") { o.push_str(&format!("{}\n", r.trim())); }
            continue;
        }
        let l = l.strip_prefix("< ").unwrap_or(l);
        if l.starts_with("HTTP/") { o.push_str(&format!("{}\n", l.trim())); in_hdr = true; continue; }
        if in_hdr && l.trim().is_empty() { in_hdr = false; continue; }
        if in_hdr { hdr(l, &mut o, &mut cookies); continue; }
        body.push(l.to_string());
    }
    if cookies > 0 { o.push_str(&format!("set-cookie x{cookies}\n")); }
    if body.is_empty() && o.is_empty() { return None; }
    let text = body.join("\n");
    match serde_json::from_str::<Value>(text.trim()) {
        Ok(v) => o.push_str(&super::jsonv::summarize(&v, true, 40)),
        Err(_) => { let b: Vec<String> = body.iter().map(|l| trunc(l, 160)).collect(); push_capped(&mut o, &b, 25); }
    }
    Some(o)
}

pub fn wget(lines: &[String]) -> Option<String> {
    let status = lines.iter().find(|l| l.contains("awaiting response")).map(|l| l.rsplit("... ").next().unwrap_or("").trim().to_string());
    let size = lines.iter().find(|l| l.starts_with("Length:")).map(|l| l.trim_start_matches("Length:").trim().to_string());
    let saved = lines.iter().find(|l| l.contains(" saved ["));
    let url = lines.iter().find(|l| l.starts_with("--")).and_then(|l| l.split_whitespace().last()).unwrap_or("");
    let errors: Vec<&String> = lines.iter().filter(|l| l.contains("ERROR") || l.contains("failed:") || l.contains("Unable to")).collect();
    if status.is_none() && saved.is_none() { return None; }
    let mut o = format!("{} {}\n", if saved.is_some() { "ok" } else { "FAILED" }, url);
    if let Some(s) = status { o.push_str(&format!("status: {s}\n")); }
    if let Some(s) = size { o.push_str(&format!("size: {s}\n")); }
    if let Some(s) = saved { o.push_str(&format!("{}\n", s.trim())); }
    for e in errors { o.push_str(&format!("{}\n", e.trim())); }
    Some(o)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn l(s: &str) -> Vec<String> { s.lines().map(String::from).collect() }
    #[test]
    fn curl_verbose_json() {
        let raw = "  % Total    % Received % Xferd  Average Speed   Time    Time     Time  Current\n                                 Dload  Upload   Total   Spent    Left  Speed\n100   120  100   120    0     0    800      0 --:--:-- --:--:-- --:--:--   805\n< HTTP/2 200\n< content-type: application/json\n< x-request-id: abcdef\n< set-cookie: a=1\n< set-cookie: b=2\n< date: Mon\n<\n{\"id\":7,\"name\":\"ana\",\"roles\":[\"a\",\"b\"]}";
        let o = curl(&l(raw)).unwrap();
        assert!(o.contains("HTTP/2 200") && o.contains("content-type: application/json") && o.contains("set-cookie x2") && o.contains("name: \"ana\""));
        assert!(!o.contains("Dload") && !o.contains("x-request-id") && !o.contains("date:") && o.len() < raw.len());
    }
    #[test]
    fn curl_plain_body_capped() {
        let mut raw = String::from("  % Total    % Received % Xferd  Average Speed   Time    Time     Time  Current\n");
        for i in 0..40 { raw.push_str(&format!("<p>line {i}</p>\n")); }
        let o = curl(&l(&raw)).unwrap();
        assert!(o.contains("+15 more") && !o.contains("% Total"));
    }
    #[test]
    fn wget_summary() {
        let raw = "--2024-03-01 10:00:00--  https://x.io/f.zip\nResolving x.io (x.io)... 1.2.3.4\nConnecting to x.io (x.io)|1.2.3.4|:443... connected.\nHTTP request sent, awaiting response... 200 OK\nLength: 1048576 (1.0M) [application/zip]\nSaving to: 'f.zip'\n\n     0K .......... .......... 100% 5.0MB/s\n\n2024-03-01 10:00:01 (5.0 MB/s) - 'f.zip' saved [1048576/1048576]";
        let o = wget(&l(raw)).unwrap();
        assert!(o.starts_with("ok https://x.io/f.zip") && o.contains("status: 200 OK") && o.contains("saved [1048576/1048576]") && !o.contains("Resolving"));
    }
    #[test]
    fn aws_json_and_s3() {
        let raw = "{\n  \"UserId\": \"AIDA123\",\n  \"Account\": \"123456789012\",\n  \"Arn\": \"arn:aws:iam::123456789012:user/ana\"\n}";
        let o = aws(&["sts".into(), "get-caller-identity".into()], &l(raw)).unwrap();
        assert!(o.contains("Account: \"123456789012\""));
        let s3 = l("                           PRE logs/\n2024-03-01 10:00:00       2048 a.txt\n2024-03-01 10:00:00    1048576 b.bin");
        let o = aws(&["s3".into(), "ls".into()], &s3).unwrap();
        assert!(o.contains("1 dirs, 2 files") && o.contains("b.bin 1.0M") && !o.contains("10:00"));
    }
}
