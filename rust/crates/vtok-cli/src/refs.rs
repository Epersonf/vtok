//! `vtok refs NAME [DIR]`: where a symbol is defined and where its name is used, across a project.
//!
//! Name-based, not type-resolved: every identifier token equal to NAME counts, grouped by the symbol that contains it.
//! Files that do not contain NAME as text are never parsed, so a query costs one pass of substring checks plus a few parses.
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use vtok_api::*;

const SKIP_DIRS: &[&str] = &["target", "node_modules", "venv", "dist", "build", "__pycache__", "vendor", "site-packages"];
const MAX_FILES: usize = 5000;
const MAX_BYTES: u64 = 1_000_000;

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    let mut entries: Vec<_> = rd.flatten().collect();
    entries.sort_by_key(|e| e.file_name());
    for e in entries {
        if out.len() >= MAX_FILES { return; }
        let name = e.file_name().to_string_lossy().to_string();
        let path = e.path();
        let Ok(ft) = e.file_type() else { continue };
        if ft.is_dir() {
            if name.starts_with('.') || SKIP_DIRS.contains(&name.as_str()) { continue; }
            walk(&path, out);
        } else if ft.is_file() && language_for_path(&name).is_ok() && e.metadata().map_or(false, |m| m.len() <= MAX_BYTES) {
            out.push(path);
        }
    }
}

fn base(id: &str) -> &str {
    let id = id.split('#').next().unwrap_or(id);
    id.rsplit('.').next().unwrap_or(id)
}

/// `class Name`, `def name`, `pub fn name`, `export const name` ...: a line that defines `name` (checked on the text).
fn defines(line: &str, name: &str) -> bool {
    let mut w = line.trim().split(|c: char| c.is_whitespace() || c == '(' || c == '<' || c == ':' || c == '{' || c == '=' || c == ';').filter(|x| !x.is_empty());
    while let Some(t) = w.next() {
        if matches!(t, "pub" | "export" | "default" | "async" | "unsafe" | "abstract" | "declare" | "extern" | "final" | "static") || t.starts_with("pub") || t.ends_with(')') { continue; }
        return matches!(t, "fn" | "struct" | "enum" | "trait" | "type" | "const" | "mod" | "class" | "def" | "function" | "interface" | "let" | "var" | "union") && w.next() == Some(name);
    }
    false
}

pub struct Report { pub text: String, pub raw_tokens: usize }

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
        let rel = f.strip_prefix(root).unwrap_or(f).display().to_string();
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
    Report { text: out, raw_tokens }
}

pub fn run(args: &[String]) -> i32 {
    let mut pos = args.iter().filter(|a| !a.starts_with("--"));
    let Some(name) = pos.next() else { eprintln!("usage: vtok refs NAME [DIR] [--max-tokens N]"); return 2 };
    let dir = pos.next().map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
    let budget = args.iter().position(|a| a == "--max-tokens").and_then(|i| args.get(i + 1)).and_then(|v| v.parse().ok()).unwrap_or(500);
    let r = find(name, &dir, budget);
    print!("{}", r.text);
    crate::usage::record("refs", r.raw_tokens.max(ntok(&r.text)), ntok(&r.text), name);
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_defs_and_code_uses_only() {
        let d = std::env::temp_dir().join(format!("vtok-refs-{}", std::process::id()));
        std::fs::create_dir_all(d.join("pkg")).unwrap();
        std::fs::write(d.join("pkg/a.py"), "def helper(x):\n    return x + 1\n\n# helper in a comment\nNOTE = \"helper in a string\"\n").unwrap();
        std::fs::write(d.join("pkg/b.py"), "from a import helper\n\ndef run():\n    return helper(2)\n").unwrap();
        std::fs::write(d.join("c.rs"), "fn other() {}\nfn go() { other(); }\n").unwrap();
        let r = find("helper", &d, 500).text;
        assert!(r.contains("1 def") && r.contains("def pkg/a.py helper L1-2"), "{r}");
        assert!(r.contains("use pkg/b.py run L3-4 @ L4"), "{r}");
        assert!(!r.contains("L4 ") || !r.contains("a.py run"), "{r}");
        assert!(!r.contains("@ L5") && !r.contains("L4,") , "comment and string mentions are not uses: {r}");
        let o = find("other", &d, 500).text;
        assert!(o.contains("def c.rs other L1") && o.contains("use c.rs go L2 @ L2"), "{o}");
        assert!(find("missing_name", &d, 500).text.contains("no definition found"));
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn defines_recognises_declarations() {
        assert!(defines("pub(crate) fn run(x: u8)", "run") && defines("class Session(Base):", "Session") && defines("export const run = 1", "run"));
        assert!(!defines("    run(1)", "run") && !defines("let x = run;", "run"));
    }
}
