//! HTML to compact readable text for `vtok browse`: headings, lists, tables, code and links survive as light Markdown;
//! scripts, styles, navigation chrome, SVG and comments are dropped; entities are decoded; blank and repeated lines collapse.

const SKIP: &[&str] = &["script", "style", "noscript", "svg", "template", "head", "iframe", "canvas", "select", "button", "nav", "footer", "aside"];
const BLOCK: &[&str] = &["p", "div", "section", "article", "main", "header", "ul", "ol", "table", "tr", "blockquote", "figure", "details", "dl", "form", "br", "hr"];

fn decode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        let tail = &rest[i..];
        let end = tail.find(';').filter(|&e| e <= 10);
        let ent = end.map(|e| &tail[1..e]);
        let rep = ent.and_then(|e| match e {
            "amp" => Some('&'), "lt" => Some('<'), "gt" => Some('>'), "quot" => Some('"'), "apos" | "#39" => Some('\''), "nbsp" => Some(' '), "mdash" => Some('—'), "ndash" => Some('–'), "hellip" => Some('…'), "copy" => Some('©'),
            n if n.starts_with("#x") => u32::from_str_radix(&n[2..], 16).ok().and_then(char::from_u32),
            n if n.starts_with('#') => n[1..].parse().ok().and_then(char::from_u32),
            _ => None,
        });
        match (rep, end) { (Some(c), Some(e)) => { out.push(c); rest = &tail[e + 1..]; } _ => { out.push('&'); rest = &tail[1..]; } }
    }
    out.push_str(rest);
    out
}

fn attr(tag: &str, name: &str) -> Option<String> {
    let low = tag.to_ascii_lowercase();
    let i = low.find(&format!("{name}="))? + name.len() + 1;
    let rest = &tag[i..];
    let (q, body) = match rest.chars().next()? { c @ ('"' | '\'') => (c, &rest[1..]), _ => (' ', rest) };
    let end = body.find(|c: char| if q == ' ' { c.is_whitespace() || c == '>' } else { c == q }).unwrap_or(body.len());
    Some(decode(&body[..end]))
}

/// Convert `html` to text. `links`: keep `(url)` after link text.
pub fn to_text(html: &str, links: bool) -> String {
    let mut out = String::new();
    let mut skip: Vec<String> = vec![];
    let mut href: Vec<Option<String>> = vec![];
    let (mut pre, mut title) = (false, String::new());
    let mut in_title = false;
    let mut i = 0;
    let b = html.as_bytes();
    while i < b.len() {
        if b[i] != b'<' {
            let j = html[i..].find('<').map_or(b.len(), |x| i + x);
            let chunk = &html[i..j];
            if in_title { title.push_str(&decode(chunk)); }
            else if skip.is_empty() {
                let t = decode(chunk);
                if pre { out.push_str(&t); } else { let c = t.split_whitespace().collect::<Vec<_>>().join(" "); if !c.is_empty() { if t.starts_with(char::is_whitespace) && !out.ends_with(['\n', ' ']) { out.push(' '); } out.push_str(&c); if t.ends_with(char::is_whitespace) { out.push(' '); } } }
            }
            i = j;
            continue;
        }
        if html[i..].starts_with("<!--") { i = html[i..].find("-->").map_or(b.len(), |x| i + x + 3); continue; }
        let Some(e) = html[i..].find('>') else { break };
        let tag = &html[i + 1..i + e];
        i += e + 1;
        let closing = tag.starts_with('/');
        let name = tag.trim_start_matches('/').split(|c: char| c.is_whitespace() || c == '/').next().unwrap_or("").to_ascii_lowercase();
        if name == "title" { in_title = !closing; continue; }
        if SKIP.contains(&name.as_str()) {
            if closing { if skip.last() == Some(&name) { skip.pop(); } } else if !tag.ends_with('/') { skip.push(name); }
            continue;
        }
        if !skip.is_empty() { continue; }
        match (name.as_str(), closing) {
            ("h1" | "h2" | "h3" | "h4" | "h5" | "h6", false) => { out.push_str("\n\n"); out.push_str(&"#".repeat(name[1..].parse().unwrap_or(1))); out.push(' '); }
            ("h1" | "h2" | "h3" | "h4" | "h5" | "h6", true) => out.push('\n'),
            ("li", false) => out.push_str("\n- "),
            ("td" | "th", true) => out.push_str(" | "),
            ("pre", false) => { pre = true; out.push_str("\n```\n"); }
            ("pre", true) => { pre = false; out.push_str("\n```\n"); }
            ("code", _) if !pre => out.push('`'),
            ("a", false) => href.push(attr(tag, "href")),
            ("a", true) => { if let Some(Some(h)) = href.pop() { if links && !h.starts_with('#') && !h.starts_with("javascript:") { out.push_str(&format!(" ({h})")); } } }
            ("img", false) => { if let Some(a) = attr(tag, "alt").filter(|a| !a.trim().is_empty()) { out.push_str(&format!("[img: {}]", a.trim())); } }
            (n, _) if BLOCK.contains(&n) => out.push('\n'),
            _ => {}
        }
    }
    let mut lines: Vec<String> = vec![];
    let mut fenced = false;
    for l in out.lines() {
        if l.trim_start().starts_with("```") { fenced = !fenced; lines.push("```".to_string()); continue; }
        let l = if fenced { l.trim_end().to_string() } else { l.trim().trim_end_matches(" |").to_string() };
        if !fenced && l.is_empty() && lines.last().map_or(true, |p| p.is_empty()) { continue; }
        if !fenced && !l.is_empty() && lines.last() == Some(&l) { continue; }
        lines.push(l);
    }
    let body = lines.join("\n").trim().to_string();
    let t = title.split_whitespace().collect::<Vec<_>>().join(" ");
    if t.is_empty() { body } else { format!("# {t}\n\n{body}") }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_becomes_light_markdown() {
        let h = "<html><head><title>My &amp; Page</title><style>a{}</style></head><body><nav><a href='/x'>Home</a></nav><h1>Hello</h1><p>Some <b>bold</b>   text &lt;ok&gt;.</p><ul><li>one</li><li><a href=\"https://e.com/a\">two</a></li></ul><pre>let x = 1;\n  y</pre><script>var a = 1;</script><!-- c --><img src=a.png alt=\"logo\"><table><tr><td>a</td><td>b</td></tr></table></body></html>";
        let t = to_text(h, true);
        assert!(t.starts_with("# My & Page"), "{t}");
        assert!(t.contains("# Hello") && t.contains("Some bold text <ok>.") && t.contains("- one") && t.contains("- two (https://e.com/a)"), "{t}");
        assert!(t.contains("```\nlet x = 1;\n  y\n```") && t.contains("[img: logo]") && t.contains("a | b"), "{t}");
        assert!(!t.contains("Home") && !t.contains("var a") && !t.contains("a{}"), "{t}");
        assert!(!to_text(h, false).contains("e.com"));
    }

    #[test]
    fn entities_and_odd_markup_do_not_panic() {
        assert_eq!(decode("a&#65;&#x42;&bogus;&amp"), "aAB&bogus;&amp");
        assert!(to_text("<p>a<br/>b</p><<>><div", false).starts_with("a\nb"));
    }
}
