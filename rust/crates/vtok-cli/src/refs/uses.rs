//! `vtok refs NAME`: the definitions of a name and the symbols that use it.
use super::walk::{base, walk};
use super::Report;
use std::collections::BTreeMap;
use std::path::Path;
use vtok_api::*;

/// `class Name`, `def name`, `pub fn name`, `export const name` ...: a line that defines `name` (checked on the text).
pub(super) fn defines(line: &str, name: &str) -> bool {
    let mut w = line.trim().split(|c: char| c.is_whitespace() || c == '(' || c == '<' || c == ':' || c == '{' || c == '=' || c == ';').filter(|x| !x.is_empty());
    while let Some(t) = w.next() {
        if matches!(t, "pub" | "export" | "default" | "async" | "unsafe" | "abstract" | "declare" | "extern" | "final" | "static") || t.starts_with("pub") || t.ends_with(')') { continue; }
        return matches!(t, "fn" | "struct" | "enum" | "trait" | "type" | "const" | "mod" | "class" | "def" | "function" | "interface" | "let" | "var" | "union") && w.next() == Some(name);
    }
    false
}

/// Defs and uses of `name` under `root`, as compact lines, within `budget` tokens.
pub fn find(name: &str, root: &Path, budget: usize) -> Report {
    let mut files = vec![];
    walk(root, &mut files);
    let mut defs: Vec<String> = vec![];
    // (file, enclosing symbol, range) -> lines
    let mut uses: BTreeMap<(String, String, String), Vec<usize>> = BTreeMap::new();
    let (mut nfiles, mut skipped, mut raw_tokens) = (0usize, 0usize, 0usize);
    for f in &files {
        let Ok(src) = std::fs::read_to_string(f) else { continue };
        if !src.contains(name) { continue; }
        let rel = f.strip_prefix(root).unwrap_or(f).display().to_string().replace(std::path::MAIN_SEPARATOR, "/");
        let Ok(l) = language_for_path(&rel) else { continue };
        let (Ok(syms), Ok(ids)) = (symbols(l.name(), &src), idents(l.name(), &src)) else { skipped += 1; continue };
        raw_tokens += ntok(&src);
        let text_lines: Vec<&str> = src.lines().collect();
        let hits: Vec<usize> = ids.iter().filter(|(n, _)| n == name).map(|(_, ln)| *ln).collect();
        if hits.is_empty() { continue; }
        nfiles += 1;
        let span = |a: usize, b: usize| if a == b { format!("L{a}") } else { format!("L{a}-{b}") };
        let is_def = |id: &str| base(id) == name && id != "use" && !id.starts_with("use#") && !id.starts_with("import");
        for (id, a, b) in &syms { if is_def(id) { defs.push(format!("def {rel} {} {}", id.split('#').next().unwrap_or(id), span(*a, *b))); } }
        for ln in hits {
            let text = text_lines.get(ln - 1).copied().unwrap_or("");
            let encl_def = syms.iter().any(|(id, a, _)| is_def(id) && *a == ln);
            if defines(text, name) && !encl_def { defs.push(format!("def {rel} {name} {} (outside any listed symbol, e.g. a class shell)", span(ln, ln))); continue; }
            // smallest symbol containing the line
            let enc = syms.iter().filter(|(_, a, b)| *a <= ln && ln <= *b).min_by_key(|(_, a, b)| b - a);
            match enc {
                Some((id, a, b)) if is_def(id) && *a == ln => {} // the definition's own name
                Some((id, a, b)) => uses.entry((rel.clone(), id.split('#').next().unwrap_or(id).to_string(), span(*a, *b))).or_default().push(ln),
                None => uses.entry((rel.clone(), "(top level)".into(), String::new())).or_default().push(ln),
            }
        }
    }
    let nuses: usize = uses.values().map(|v| v.len()).sum();
    let mut out = format!("# refs {name}: {} def, {nuses} uses in {nfiles} files (by name, not type-resolved)\n", defs.len());
    if defs.is_empty() { out.push_str("# no definition found in this tree (external, generated, or a different spelling)\n"); }
    if defs.len() > 1 { out.push_str(&format!("# {} definitions share this name: uses are not told apart\n", defs.len())); }
    if skipped > 0 { out.push_str(&format!("# {skipped} files skipped (syntax errors)\n")); }
    for d in &defs { out.push_str(d); out.push('\n'); }
    let mut shown = 0;
    let total = uses.len();
    for ((file, sym, range), lines) in &uses {
        let ls: Vec<String> = lines.iter().map(|l| format!("L{l}")).collect();
        let line = format!("use {file} {sym}{}{} @ {}\n", if range.is_empty() { "" } else { " " }, range, ls.join(","));
        if ntok(&(out.clone() + &line)) > budget { break; }
        out.push_str(&line);
        shown += 1;
    }
    if shown < total { out.push_str(&format!("# {} more use sites not shown (raise max_tokens or narrow with a directory)\n", total - shown)); }
    Report { text: out, raw_tokens, top: vec![] }
}
