//! `vtok proxy`: a local HTTP proxy (127.0.0.1 only) in front of an Anthropic- or OpenAI-compatible API. A request whose
//! `model` is "auto" gets the model vtok's router picks for its last user message; every other request passes through
//! untouched, and responses (including SSE streams) are relayed as they arrive. Point the agent at it with
//! `ANTHROPIC_BASE_URL=http://127.0.0.1:PORT` (or the client's base-URL setting) and choose the model name `auto`.
use serde_json::Value;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};

const MAX_BODY: usize = 32 * 1024 * 1024;

/// Text of the last user message, for both API styles (string content or a list of text blocks).
fn last_user_text(v: &Value) -> String {
    let Some(m) = v.get("messages").and_then(|m| m.as_array()).and_then(|a| a.iter().rev().find(|m| m.get("role").and_then(|r| r.as_str()) == Some("user"))) else { return String::new() };
    match m.get("content") {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Array(parts)) => parts.iter().filter_map(|p| p.get("text").and_then(|t| t.as_str())).collect::<Vec<_>>().join("\n"),
        _ => String::new(),
    }
}

/// Rewrite the body when its model is "auto"; returns (new body, route line) or None to forward as is.
pub fn route_body(body: &[u8]) -> Option<(Vec<u8>, String)> {
    let mut v: Value = serde_json::from_slice(body).ok()?;
    if !matches!(v.get("model").and_then(|m| m.as_str()), Some("auto") | Some("vtok-auto")) { return None; }
    let r = crate::route::classify(&last_user_text(&v), 0);
    v["model"] = Value::String(r.model.clone());
    Some((serde_json::to_vec(&v).ok()?, format!("{}:{}", r.tier, r.model)))
}

fn handle(mut s: TcpStream, upstream: &str) -> std::io::Result<()> {
    let mut rd = BufReader::new(s.try_clone()?);
    let mut line = String::new();
    rd.read_line(&mut line)?;
    let (method, path) = { let mut it = line.split_whitespace(); (it.next().unwrap_or("").to_string(), it.next().unwrap_or("/").to_string()) };
    let (mut headers, mut len) = (vec![], 0usize);
    loop {
        let mut h = String::new();
        if rd.read_line(&mut h)? == 0 || h.trim().is_empty() { break; }
        if let Some((k, v)) = h.trim_end().split_once(':') {
            if k.eq_ignore_ascii_case("content-length") { len = v.trim().parse().unwrap_or(0); }
            else if !["host", "connection", "accept-encoding", "transfer-encoding", "proxy-connection", "keep-alive", "expect"].contains(&k.to_ascii_lowercase().as_str()) { headers.push((k.to_string(), v.trim().to_string())); }
        }
    }
    if len > MAX_BODY { return s.write_all(b"HTTP/1.1 413 Payload Too Large\r\nConnection: close\r\n\r\n"); }
    let mut body = vec![0u8; len];
    rd.read_exact(&mut body)?;
    let routed = if method == "POST" { route_body(&body) } else { None };
    let tag = routed.as_ref().map(|r| r.1.clone());
    if let Some((b, t)) = routed { body = b; crate::usage::record("route", 0, 0, &t); }
    let url = format!("{}{}", upstream.trim_end_matches('/'), path);
    let agent: ureq::Agent = ureq::Agent::config_builder().http_status_as_error(false).timeout_global(Some(std::time::Duration::from_secs(900))).build().into();
    let mut resp = match method.as_str() {
        "POST" | "PUT" | "PATCH" => { let mut r = agent.post(&url); for (k, v) in &headers { r = r.header(k, v); } r.send(&body[..]) }
        _ => { let mut r = agent.get(&url); for (k, v) in &headers { r = r.header(k, v); } r.call() }
    };
    let resp = match resp.as_mut() { Ok(r) => r, Err(e) => { let m = format!("vtok proxy: {e}"); return s.write_all(format!("HTTP/1.1 502 Bad Gateway\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{m}", m.len()).as_bytes()); } };
    let mut head = format!("HTTP/1.1 {} {}\r\nConnection: close\r\n", resp.status().as_u16(), resp.status().canonical_reason().unwrap_or(""));
    for (k, v) in resp.headers() {
        if ["content-length", "transfer-encoding", "connection", "content-encoding"].contains(&k.as_str()) { continue; }
        if let Ok(v) = v.to_str() { head.push_str(&format!("{k}: {v}\r\n")); }
    }
    if let Some(t) = tag { head.push_str(&format!("x-vtok-route: {t}\r\n")); }
    s.write_all(head.as_bytes())?;
    s.write_all(b"\r\n")?;
    let mut r = resp.body_mut().as_reader();
    let mut buf = [0u8; 8192];
    loop { let n = r.read(&mut buf)?; if n == 0 { break; } s.write_all(&buf[..n])?; s.flush()?; }
    Ok(())
}

pub fn run(args: &[String]) -> i32 {
    let val = |n: &str| args.iter().position(|a| a == n).and_then(|i| args.get(i + 1)).cloned();
    let port: u16 = val("--port").and_then(|p| p.parse().ok()).unwrap_or(8787);
    let upstream = val("--upstream").or_else(|| std::env::var("VTOK_UPSTREAM").ok()).unwrap_or_else(|| "https://api.anthropic.com".into());
    let l = match TcpListener::bind(("127.0.0.1", port)) { Ok(l) => l, Err(e) => { eprintln!("cannot listen on 127.0.0.1:{port}: {e}"); return 1; } };
    eprintln!("vtok proxy on http://127.0.0.1:{port} -> {upstream}  (model \"auto\" is routed; everything else passes through)");
    for c in l.incoming().flatten() {
        let up = upstream.clone();
        std::thread::spawn(move || { let _ = handle(c, &up); });
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_auto_is_rewritten() {
        let b = br#"{"model":"auto","max_tokens":10,"messages":[{"role":"user","content":[{"type":"text","text":"fix the typo in the README"}]}]}"#;
        let (nb, tag) = route_body(b).unwrap();
        let v: Value = serde_json::from_slice(&nb).unwrap();
        assert!(v["model"].as_str().unwrap().contains("haiku") && tag.starts_with("small:") && v["max_tokens"] == 10);
        assert!(route_body(br#"{"model":"claude-opus-5-5","messages":[]}"#).is_none() && route_body(b"not json").is_none());
        let o = br#"{"model":"vtok-auto","messages":[{"role":"user","content":"Investigate the root cause of this deadlock and refactor the locking design across the whole service"}]}"#;
        assert!(route_body(o).unwrap().1.starts_with("large:"));
    }
}
