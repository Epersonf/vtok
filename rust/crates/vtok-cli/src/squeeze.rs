//! `vtok squeeze FILE`: shrink an instructions / memory file (CLAUDE.md, AGENTS.md, notes) that an agent re-reads every
//! session. Deterministic, no model: trims whitespace, drops decoration and filler phrases from the prose. Code blocks,
//! inline code, URLs, headings, tables and list structure are left exactly as they are.
use vtok_api::ntok;

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

fn prose_line(line: &str) -> String {
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

pub fn squeeze(text: &str) -> String {
    let mut out: Vec<String> = vec![];
    let mut fence: Option<String> = None;
    let mut blank = 0;
    let mut front = false;
    for (n, raw) in text.lines().enumerate() {
        let t = raw.trim_start();
        if n == 0 && raw.trim() == "---" { front = true; out.push(raw.to_string()); continue; }
        if front { if raw.trim() == "---" { front = false; } out.push(raw.to_string()); continue; }
        if let Some(f) = &fence {
            out.push(raw.trim_end().to_string());
            if t.starts_with(f.as_str()) { fence = None; }
            continue;
        }
        if t.starts_with("```") || t.starts_with("~~~") { fence = Some(t[..3].to_string()); blank = 0; out.push(raw.trim_end().to_string()); continue; }
        if raw.trim().is_empty() { blank += 1; if blank == 1 { out.push(String::new()); } continue; }
        let indented_code = raw.starts_with("    ") && !t.starts_with(['-', '*', '+']) && !t.chars().next().map_or(false, |c| c.is_ascii_digit());
        let is_rule = t.len() >= 3 && t.chars().all(|c| matches!(c, '-' | '*' | '_' | ' ')) && !t.starts_with("| ");
        if is_rule { continue; }
        blank = 0;
        if indented_code || t.starts_with('|') || t.starts_with('<') { out.push(raw.trim_end().to_string()); continue; }
        out.push(prose_line(raw));
    }
    while out.last().map_or(false, |l| l.is_empty()) { out.pop(); }
    out.join("\n") + "\n"
}

/// What a rewrite must keep: fenced code, inline code, URLs and headings. Returns what is missing from `new`.
pub fn missing(old: &str, new: &str) -> Vec<String> {
    let mut keep: Vec<String> = vec![];
    let mut fence = false;
    let mut block = String::new();
    for l in old.lines() {
        if l.trim_start().starts_with("```") { if fence { block.push_str(l.trim()); keep.push(std::mem::take(&mut block)); } fence = !fence; if fence { block.push_str(l.trim()); block.push('\n'); } continue; }
        if fence { block.push_str(l.trim_end()); block.push('\n'); continue; }
        if l.starts_with('#') { keep.push(l.trim().to_string()); }
        for (i, seg) in l.split('`').enumerate() { if i % 2 == 1 && !seg.is_empty() { keep.push(format!("`{seg}`")); } }
        for w in l.split_whitespace() { if let Some(p) = w.find("http") { let u = w[p..].trim_end_matches([')', ',', '.', ';', '>', '"']); if u.contains("://") { keep.push(u.to_string()); } } }
    }
    let squash = |t: &str| t.split_whitespace().collect::<Vec<_>>().join(" ");
    let hay = squash(new);
    keep.sort(); keep.dedup();
    keep.into_iter().filter(|k| !hay.contains(&squash(k))).collect()
}

const REWRITE_PROMPT: &str = "Rewrite the file below so an AI agent reads it in fewer tokens with no loss of instructions. Rules: keep every command, path, flag, number, URL, heading and code block exactly as written; delete repetition, examples that restate a rule, and marketing or background prose; use short imperative sentences, no filler. Do not add anything. Save the result to a new file and run `vtok squeeze --check ORIGINAL NEW` to verify nothing protected was lost.\n\n";

pub fn run(args: &[String]) -> i32 {
    let pos: Vec<&String> = args.iter().filter(|a| !a.starts_with("--")).collect();
    if args.iter().any(|a| a == "--check") {
        let (Some(o), Some(n)) = (pos.first(), pos.get(1)) else { eprintln!("usage: vtok squeeze --check ORIGINAL NEW"); return 2 };
        let (Ok(old), Ok(new)) = (std::fs::read_to_string(o), std::fs::read_to_string(n)) else { eprintln!("cannot read one of the files"); return 1 };
        let m = missing(&old, &new);
        let (a, b) = (ntok(&old), ntok(&new));
        println!("{a} -> {b} tokens ({:.0}% less)", 100.0 * (1.0 - b as f64 / a.max(1) as f64));
        if m.is_empty() { println!("ok: every code block, inline code, URL and heading is still there"); return 0; }
        println!("MISSING {} protected item(s):", m.len());
        for x in m.iter().take(20) { println!("  {}", x.lines().next().unwrap_or("")); }
        return 1;
    }
    if args.iter().any(|a| a == "--rewrite") {
        let Some(p) = pos.first() else { eprintln!("usage: vtok squeeze --rewrite FILE   (prints a rewrite request for your agent)"); return 2 };
        let Ok(t) = std::fs::read_to_string(p) else { eprintln!("{p}: cannot read as text"); return 1 };
        print!("{REWRITE_PROMPT}--- {p} ---\n{t}");
        return 0;
    }
    let write = args.iter().any(|a| a == "--write");
    let Some(path) = pos.first() else { eprintln!("usage: vtok squeeze FILE [--write] | --rewrite FILE | --check ORIGINAL NEW"); return 2 };
    let Ok(text) = std::fs::read_to_string(path) else { eprintln!("{path}: cannot read as text"); return 1 };
    let new = squeeze(&text);
    let (a, b) = (ntok(&text), ntok(&new));
    if write {
        if new == text { println!("{path}: nothing to squeeze"); return 0; }
        let bak = format!("{path}.orig");
        if !std::path::Path::new(&bak).exists() { if let Err(e) = std::fs::write(&bak, &text) { eprintln!("{bak}: {e}"); return 1; } }
        if let Err(e) = std::fs::write(path, &new) { eprintln!("{path}: {e}"); return 1; }
        println!("{path}: {a} -> {b} tokens ({:.0}% less); original kept as {bak}", 100.0 * (1.0 - b as f64 / a.max(1) as f64));
    } else {
        print!("{new}");
        eprintln!("# {a} -> {b} tokens ({:.0}% less); nothing written (use --write)", 100.0 * (1.0 - b as f64 / a.max(1) as f64));
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prose_shrinks_but_code_and_structure_stay() {
        let src = "# Title  \n\n\n\nPlease note that you **must** run `make  sure to  go` in order to build.\n\n---\n\n```sh\n  just   run   this   \n```\n\n| a | b |\n|---|---|\n\n- Basically  keep   it simple, see https://x.y/z?a=1\n";
        let out = squeeze(src);
        assert!(out.contains("You must run `make  sure to  go` to build."), "{out}");
        assert!(out.contains("```sh\n  just   run   this\n```"), "code block kept (only trailing blanks trimmed): {out}");
        assert!(out.contains("| a | b |\n|---|---|"), "{out}");
        assert!(out.contains("- Keep it simple, see https://x.y/z?a=1"), "{out}");
        assert!(!out.contains("\n\n\n") && !out.contains("\n---\n"), "{out}");
        assert_eq!(squeeze(&out), out, "idempotent");
    }

    #[test]
    fn check_reports_lost_protected_items() {
        let old = "# Build\nRun `make all` then see https://a.b/c.\n```sh\ncargo test\n```\n";
        assert!(missing(old, "# Build\nRun `make all`, see https://a.b/c.\n```sh\ncargo test\n```\n").is_empty());
        let m = missing(old, "# Build\nRun make.\n");
        assert!(m.iter().any(|x| x.contains("make all")) && m.iter().any(|x| x.contains("https://a.b/c")) && m.iter().any(|x| x.contains("cargo test")), "{m:?}");
    }

    #[test]
    fn word_boundaries_and_capitals() {
        assert_eq!(prose_line("It is a justification, not just a rule."), "It is a justification, not a rule.");
        assert_eq!(prose_line("very good"), "Good");
        assert_eq!(prose_line("we utilize it"), "we use it");
    }
}
