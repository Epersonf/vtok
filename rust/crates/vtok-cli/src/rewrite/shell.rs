//! Reading a shell line the way the rewrite needs it: where its segments are, whether one has a pipe, a redirect or a
//! substitution, and what is a leading `cd DIR &&` prefix.

/// Split a line at top-level `&&`, `||` and `;`. Returns (segments with their trailing separator, ok). `ok` is false when the
/// line has something that makes segment-wise rewriting unsafe (background `&`, heredoc, newline, unbalanced quote).
pub(super) fn segments(cmd: &str) -> Option<Vec<(String, &'static str)>> {
    let b: Vec<char> = cmd.chars().collect();
    let (mut out, mut cur, mut i) = (vec![], String::new(), 0);
    let mut q: Option<char> = None;
    while i < b.len() {
        let c = b[i];
        if let Some(qc) = q {
            cur.push(c);
            if c == '\\' && qc == '"' && i + 1 < b.len() { cur.push(b[i + 1]); i += 1; } else if c == qc { q = None; }
        } else if c == '\'' || c == '"' { q = Some(c); cur.push(c); }
        else if c == '\\' && i + 1 < b.len() { cur.push(c); cur.push(b[i + 1]); i += 1; }
        else if c == '\n' { return None; }
        else if c == '&' && b.get(i + 1) == Some(&'&') { out.push((std::mem::take(&mut cur), "&&")); i += 1; }
        else if c == '|' && b.get(i + 1) == Some(&'|') { out.push((std::mem::take(&mut cur), "||")); i += 1; }
        else if c == ';' { out.push((std::mem::take(&mut cur), ";")); }
        else if c == '&' || (c == '<' && b.get(i + 1) == Some(&'<')) { return None; }
        else { cur.push(c); }
        i += 1;
    }
    if q.is_some() { return None; }
    out.push((cur, ""));
    Some(out)
}

/// True when the segment has an unquoted pipe, redirect, backtick, substitution or grouping.
pub(super) fn has_shell_meta(seg: &str) -> bool {
    let mut q: Option<char> = None;
    let mut esc = false;
    let b: Vec<char> = seg.chars().collect();
    for (i, &c) in b.iter().enumerate() {
        if esc { esc = false; continue; }
        match q {
            Some('\'') => { if c == '\'' { q = None; } }
            Some(_) => { if c == '\\' { esc = true; } else if c == '"' { q = None; } else if c == '`' || (c == '$' && b.get(i + 1) == Some(&'(')) { return true; } }
            None => match c {
                '\\' => esc = true,
                '\'' | '"' => q = Some(c),
                '|' | '<' | '>' | '`' | '(' | ')' | '{' | '}' => return true,
                '$' if b.get(i + 1) == Some(&'(') => return true,
                _ => {}
            },
        }
    }
    false
}

/// Split a leading `cd DIR && ` / `export X=y && ` prefix (agents add it to nearly every command) from the command itself.
pub fn split_prefix(cmd: &str) -> (&str, &str) {
    let mut rest = cmd.trim_start();
    let mut used = 0;
    while let Some(i) = rest.find("&&") {
        let head = rest[..i].trim();
        let simple = |h: &str| !h.is_empty() && !h.contains(['|', ';', '<', '>', '`', '\n', '(', ')', '{', '}', '&']) && !h.contains("$(");
        if (head.starts_with("cd ") || head.starts_with("export ")) && simple(head) {
            used += cmd.trim_start().len() - rest.len() + i + 2;
            rest = &rest[i + 2..];
        } else { break; }
    }
    let off = cmd.len() - cmd.trim_start().len() + used;
    (&cmd[..off.min(cmd.len())], cmd[off.min(cmd.len())..].trim_start())
}
