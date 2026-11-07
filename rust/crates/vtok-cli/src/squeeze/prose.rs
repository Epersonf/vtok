//! Prose rewriting of one line: filler phrases out, emphasis marks out, runs of spaces collapsed. Inline code and URLs stay.

/// (phrase, replacement), lowercase, matched on word boundaries outside code.
const PHRASES: &[(&str, &str)] = &[
    ("please note that ", ""), ("note that ", ""), ("it is important to ", ""), ("it is worth noting that ", ""), ("keep in mind that ", ""),
    ("in order to ", "to "), ("due to the fact that ", "because "), ("at this point in time", "now"), ("in the event that ", "if "),
    ("for the purpose of ", "for "), ("with regard to ", "about "), ("with respect to ", "about "), ("a number of ", "several "),
    ("is able to ", "can "), ("are able to ", "can "), ("as well as ", "and "), ("in addition to ", "besides "), ("prior to ", "before "),
    ("subsequent to ", "after "), ("utilize ", "use "), ("utilise ", "use "), ("basically ", ""), ("essentially ", ""), ("simply ", ""),
    ("actually ", ""), ("really ", ""), ("very ", ""), ("just ", ""), ("that being said, ", ""), ("needless to say, ", ""),
    ("it should be noted that ", ""), ("make sure that you ", "ensure you "), ("make sure to ", "ensure you "), ("you should always ", "always "),
];

fn is_word(c: char) -> bool { c.is_alphanumeric() || c == '_' || c == '-' }

fn replace_phrases(seg: &str) -> String {
    let mut s = seg.to_string();
    for (from, to) in PHRASES {
        let mut out = String::new();
        let mut i = 0;
        let low = s.to_lowercase();
        if low.len() != s.len() { return s; } // case mapping changed byte lengths: leave this segment alone
        while let Some(p) = low[i..].find(from) {
            let at = i + p;
            let boundary_ok = s[..at].chars().next_back().map_or(true, |c| !is_word(c));
            out.push_str(&s[i..at]);
            if boundary_ok {
                // a removed sentence opener hands its capital to the next word
                let pre = out.trim_end();
                let opener = pre.is_empty() || pre.ends_with(['.', '!', '?', ':']) || matches!(pre, "-" | "*" | "+") || (pre.ends_with('.') && pre[..pre.len() - 1].chars().all(|c| c.is_ascii_digit()));
                let rest = s[at + from.len()..].trim_start_matches(' ');
                if to.is_empty() && opener {
                    let mut cs = rest.chars();
                    if let Some(c) = cs.next() { out.extend(c.to_uppercase()); out.push_str(cs.as_str()); i = s.len(); continue; }
                }
                out.push_str(to);
                i = at + from.len();
            } else { out.push_str(&s[at..at + from.len()]); i = at + from.len(); }
        }
        out.push_str(&s[i..]);
        s = out;
    }
    s
}

fn strip_bold(seg: &str) -> String {
    let mut s = seg.replace("**", "");
    // __x__ emphasis only when it wraps whole words (leave snake__case__names alone)
    if s.starts_with("__") && s.contains(' ') { s = s.replace("__", ""); }
    s
}

pub(super) fn prose_line(line: &str) -> String {
    // split on backticks: even segments are prose, odd segments are code and stay verbatim
    let mut out = String::new();
    for (i, seg) in line.split('`').enumerate() {
        if i > 0 { out.push('`'); }
        if i % 2 == 1 { out.push_str(seg); continue; }
        // URLs are swapped for placeholders while the phrases are replaced
        let mut urls: Vec<&str> = vec![];
        let masked: Vec<String> = seg.split(' ').map(|w| if w.contains("://") { urls.push(w); format!("\u{1}{}\u{1}", urls.len() - 1) } else { w.to_string() }).collect();
        let mut t = replace_phrases(&masked.join(" "));
        for (k, u) in urls.iter().enumerate() { t = t.replace(&format!("\u{1}{k}\u{1}"), u); }
        t = strip_bold(&t);
        // collapse runs of spaces inside the prose (keep leading indentation)
        let lead = t.len() - t.trim_start().len();
        let mut body = String::new();
        let mut prev_space = false;
        for c in t[lead..].chars() { if c == ' ' { if !prev_space { body.push(c); } prev_space = true; } else { prev_space = false; body.push(c); } }
        out.push_str(&t[..lead]);
        out.push_str(&body);
    }
    out.trim_end().to_string()
}
