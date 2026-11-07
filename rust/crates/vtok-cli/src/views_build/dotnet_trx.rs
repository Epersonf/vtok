//! Visual Studio .trx test result XML (`dotnet test --logger trx`, or a trx file printed): counters + failed tests.
use super::{cap, finish, trunc};
use regex::Regex;

pub fn view(lines: &[String]) -> Option<String> {
    let text = lines.join("\n");
    if !text.contains("<TestRun") { return None; }
    let attr = |tag: &str, name: &str| -> Option<String> {
        let t = Regex::new(&format!(r"<{tag}\b[^>]*>")).unwrap().find(&text)?.as_str().to_string();
        Regex::new(&format!(r#"\b{name}="([^"]*)""#)).unwrap().captures(&t).map(|c| c[1].to_string())
    };
    let n = |k: &str| attr("Counters", k).unwrap_or_else(|| "0".into());
    let mut out = Vec::new();
    let unit = Regex::new(r#"(?s)<UnitTestResult([^>]*?)(?:/>|>(.*?)</UnitTestResult>)"#).unwrap();
    let (name_re, out_re) = (Regex::new(r#"testName="([^"]*)""#).unwrap(), Regex::new(r#"outcome="([^"]*)""#).unwrap());
    let msg = Regex::new(r"(?s)<Message>(.*?)</Message>").unwrap();
    let stack = Regex::new(r"(?s)<StackTrace>(.*?)</StackTrace>").unwrap();
    let mut failed = Vec::new();
    for c in unit.captures_iter(&text) {
        if out_re.captures(&c[1]).map_or(true, |o| &o[1] != "Failed") { continue; }
        let body = c.get(2).map_or("", |m| m.as_str());
        let name = name_re.captures(&c[1]).map_or(String::new(), |m| m[1].to_string());
        failed.push(format!("FAIL {name}"));
        if let Some(m) = msg.captures(body) { failed.push(format!("  {}", trunc(&unescape(m[1].trim()).replace('\n', " | "), 300))); }
        if let Some(s) = stack.captures(body) { if let Some(l) = s[1].lines().map(str::trim).find(|l| !l.is_empty()) { failed.push(format!("  {}", trunc(&unescape(l), 200))); } }
    }
    let outcome = attr("ResultSummary", "outcome").unwrap_or_default();
    out.push(format!("trx {outcome}: {} total, {} passed, {} failed, {} errors", n("total"), n("passed"), n("failed"), n("error")));
    cap(&mut out, &failed, 40);
    finish(out, lines)
}

fn unescape(s: &str) -> String {
    s.replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&apos;", "'").replace("&amp;", "&")
}
