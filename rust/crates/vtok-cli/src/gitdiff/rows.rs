//! A file's displayable lines ("rows"), each with its token cost, before the budget decides how many to show:
//! hunk headers (`@@ ... @@ symbol`), changed lines, context trimmed to `ctx` lines around changes, the
//! "\ No newline" marker after the line it annotates, long lines cut (with the cut stated), and hunks identical to
//! an earlier one replaced by a pointer to it.
use super::model::*;
use std::collections::HashMap;
use vtok_api::ntok;

pub const LINE_MAX: usize = 220;
const LINE_KEEP: usize = 160;

#[derive(Clone)]
pub struct Row { pub text: String, pub changed: usize, pub head: bool, pub cost: usize }

pub fn row(text: String, changed: usize, head: bool) -> Row { let cost = ntok(&text) + 1; Row { text, changed, head, cost } }

pub fn cut(l: &str) -> String {
    let n = l.chars().count();
    if n <= LINE_MAX { return l.to_string(); }
    format!("{}…[+{} chars]", l.chars().take(LINE_KEEP).collect::<String>(), n - LINE_KEEP)
}

fn short_head(h: &str) -> String {
    let w = h.bytes().take_while(|&b| b == b'@').count();
    let close = "@".repeat(w);
    match h[w..].find(&close) {
        Some(p) => {
            let sym = h[w + p + w..].trim();
            let sym: String = sym.chars().take(80).collect();
            format!("{}{}{}", &h[..w + p + w], if sym.is_empty() { "" } else { " " }, sym)
        }
        None => h.to_string(),
    }
}

/// Rows of a modified/renamed file. `seen` maps a hunk's changed lines to where they were first shown.
pub fn hunk_rows(f: &FileDiff, ctx: usize, seen: &mut HashMap<String, String>) -> Vec<Row> {
    let mut out = vec![];
    for h in &f.hunks {
        let kinds: Vec<Kind> = h.body.iter().map(|l| kind(l, h.width)).collect();
        let changed: Vec<&String> = h.body.iter().zip(&kinds).filter(|(_, k)| matches!(k, Kind::Add | Kind::Del)).map(|(l, _)| l).collect();
        if changed.is_empty() { continue; }
        let head = short_head(&h.head);
        let key = changed.iter().map(|s| s.as_str()).collect::<Vec<_>>().join("\n");
        if changed.len() >= 2 || key.len() > 60 {
            if let Some(at) = seen.get(&key) {
                out.push(row(head, 0, true));
                out.push(row(format!("  (same {} changed lines as {at})", changed.len()), changed.len(), false));
                continue;
            }
            seen.insert(key, format!("{} {}", f.path(), head.split(" @@").next().unwrap_or("")));
        }
        out.push(row(head, 0, true));
        let near = |i: usize| (i.saturating_sub(ctx)..=(i + ctx).min(kinds.len() - 1)).any(|j| matches!(kinds[j], Kind::Add | Kind::Del));
        for (i, (l, k)) in h.body.iter().zip(&kinds).enumerate() {
            match k {
                Kind::Add | Kind::Del => out.push(row(cut(l), 1, false)),
                Kind::NoNl => if i > 0 && (matches!(kinds[i - 1], Kind::Add | Kind::Del) || ctx > 0) { out.push(row(l.clone(), 0, false)); },
                Kind::Ctx => if ctx > 0 && near(i) { out.push(row(cut(l), 0, false)); },
            }
        }
    }
    out
}

/// Rows of an added (or deleted) file: its lines with the `+`/`-` marker, no hunk header.
pub fn content_rows(f: &FileDiff) -> Vec<Row> {
    f.hunks.iter().flat_map(|h| h.body.iter().map(move |l| (l, kind(l, h.width))))
        .filter(|(_, k)| matches!(k, Kind::Add | Kind::Del | Kind::NoNl))
        .map(|(l, k)| row(cut(l), (k != Kind::NoNl) as usize, false)).collect()
}

/// Names of the symbols in a new file, from vtok's own outline (tree-sitter), when the language is supported.
pub fn symbols_line(f: &FileDiff) -> Option<String> {
    let lang = vtok_api::language_for_path(f.path()).ok()?;
    let src: String = f.hunks.iter().flat_map(|h| h.body.iter()).filter(|l| l.starts_with('+')).map(|l| format!("{}\n", &l[1..])).collect();
    let syms = vtok_api::symbols(lang.name(), &src).ok()?;
    let names: Vec<String> = syms.iter().filter(|(id, _, _)| !(id == "use" || id.starts_with("use#") || id.starts_with("import") || id.starts_with("ERROR")))
        .map(|(id, a, _)| format!("{id}@{a}")).collect();
    if names.is_empty() { return None; }
    let n = names.len();
    let shown = names.into_iter().take(40).collect::<Vec<_>>().join(" ");
    Some(format!("  symbols: {shown}{}", if n > 40 { format!(" … +{}", n - 40) } else { String::new() }))
}
