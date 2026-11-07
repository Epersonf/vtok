//! `vtok browse URL`: fetch a web page and show it as compact text (see html_text.rs). The whole converted page is kept
//! under an id, so `vtok output ID --grep ...` reads any part of it. Only http(s); private and loopback hosts are refused
//! unless `--allow-private` (the agent that calls this is not the one who decides what the machine may reach).
use vtok_api::ntok;

const MAX_BYTES: u64 = 5_000_000;

fn private_host(url: &str) -> bool {
    let rest = url.split("://").nth(1).unwrap_or("");
    let auth = rest.split(['/', '?', '#']).next().unwrap_or("");
    let host = auth.rsplit('@').next().unwrap_or("");
    let host = if host.starts_with('[') { host.trim_start_matches('[').split(']').next().unwrap_or("") } else { host.split(':').next().unwrap_or("") }.to_ascii_lowercase();
    host == "localhost" || host.ends_with(".localhost") || host.ends_with(".local") || host.ends_with(".internal") || host == "::1" || host == "0.0.0.0"
        || host.starts_with("127.") || host.starts_with("10.") || host.starts_with("192.168.") || host.starts_with("169.254.") || host.starts_with("fe80")
        || host.strip_prefix("172.").and_then(|r| r.split('.').next()).and_then(|n| n.parse::<u8>().ok()).map_or(false, |n| (16..=31).contains(&n))
}

pub struct Opts { pub budget: usize, pub links: bool, pub allow_private: bool }

/// The compact view of `url` (header line, text, pointer to the rest).
pub fn page(url: &str, o: &Opts) -> Result<String, String> {
    if !(url.starts_with("http://") || url.starts_with("https://")) { return Err("only http:// and https:// URLs".into()); }
    if !o.allow_private && private_host(url) { return Err("private, loopback and link-local hosts are refused (use --allow-private)".into()); }
    let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(std::time::Duration::from_secs(20))).user_agent("vtok-browse").build().into();
    let mut resp = agent.get(url).call().map_err(|e| format!("{url}: {e}"))?;
    let ct = resp.headers().get("content-type").and_then(|v| v.to_str().ok()).unwrap_or("").to_ascii_lowercase();
    let raw = resp.body_mut().with_config().limit(MAX_BYTES).read_to_string().map_err(|e| format!("{url}: {e}"))?;
    let text = if ct.contains("html") || raw.trim_start().starts_with('<') { crate::html_text::to_text(&raw, o.links) } else { raw.clone() };
    Ok(render(url, &raw, &text, o.budget))
}

fn render(url: &str, raw: &str, text: &str, budget: usize) -> String {
    let (raw_tok, full_tok) = (ntok(raw), ntok(text));
    let id = crate::shell::store(&(text.to_string() + "\n"));
    let mut shown = String::new();
    let mut cut = false;
    for l in text.lines() {
        if ntok(&shown) + ntok(l) + 1 > budget { cut = true; break; }
        shown.push_str(l);
        shown.push('\n');
    }
    let head = format!("# browse {url}: {raw_tok} raw tokens -> {full_tok}{}\n", id.as_ref().map(|i| format!(" id={i}")).unwrap_or_default());
    let foot = match (&id, cut) { (Some(i), true) => format!("# more: vtok output {i} --grep TEXT\n"), (None, true) => "# (truncated)\n".to_string(), _ => String::new() };
    let out = format!("{head}{shown}{foot}");
    crate::usage::record("browse", raw_tok, ntok(&out), url);
    out
}

pub fn run(args: &[String]) -> i32 {
    let flag = |n: &str| args.iter().any(|a| a == n);
    let budget = args.iter().position(|a| a == "--max-tokens").and_then(|i| args.get(i + 1)).and_then(|v| v.parse().ok()).unwrap_or(3000);
    let Some(url) = args.iter().enumerate().find(|(i, a)| !a.starts_with("--") && (*i == 0 || args[i - 1] != "--max-tokens")).map(|(_, a)| a) else { eprintln!("usage: vtok browse URL [--max-tokens N] [--no-links] [--allow-private]"); return 2 };
    match page(url, &Opts { budget, links: !flag("--no-links"), allow_private: flag("--allow-private") }) {
        Ok(t) => { print!("{t}"); 0 }
        Err(e) => { eprintln!("{e}"); 1 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn private_hosts_and_schemes_are_refused() {
        let o = Opts { budget: 100, links: true, allow_private: false };
        for u in ["http://localhost:8080/", "https://127.0.0.1/x", "http://192.168.1.5/", "http://10.0.0.1", "http://172.20.0.1/", "http://[::1]/", "http://169.254.169.254/latest/meta-data", "file:///etc/passwd", "ftp://x"] { assert!(page(u, &o).is_err(), "{u}"); }
        assert!(!private_host("https://example.com/") && !private_host("http://172.15.0.1/") && !private_host("https://user@example.org:8443/a"));
    }

    #[test]
    fn long_pages_are_cut_at_the_budget() {
        let text: String = (0..400).map(|i| format!("line number {i}\n")).collect();
        let out = render("http://x", &text, &text, 100);
        assert!(out.starts_with("# browse http://x") && out.lines().count() < 60 && out.contains("vtok output"), "{out}");
    }
}
