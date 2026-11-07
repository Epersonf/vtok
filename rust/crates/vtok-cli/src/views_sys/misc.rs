//! ast-grep matches and the generic file dump (cat of big source files).
use super::util::*;

/// ast-grep: matches grouped per file, capped. Handles `path:line:col:text` and header + `N|` gutter forms.
pub fn astgrep(lines: &[String]) -> Option<String> {
    let gutter = regex::Regex::new(r"^\s*(\d+)\s*[│|]\s?(.*)$").ok()?;
    let inline = regex::Regex::new(r"^(.+?):(\d+)(?::\d+)?:\s?(.*)$").ok()?;
    let mut files: Vec<(String, Vec<String>)> = vec![];
    for l in lines {
        if let Some(c) = gutter.captures(l) {
            if let Some(f) = files.last_mut() { f.1.push(format!("{}: {}", &c[1], trunc(c[2].trim(), 120))); }
        } else if let Some(c) = inline.captures(l) {
            let name = c[1].to_string();
            let row = format!("{}: {}", &c[2], trunc(c[3].trim(), 120));
            match files.iter_mut().find(|f| f.0 == name) { Some(f) => f.1.push(row), None => files.push((name, vec![row])) }
        } else if !l.trim().is_empty() && !l.contains(' ') && (l.contains('/') || l.contains('.')) {
            files.push((l.trim().to_string(), vec![]));
        }
    }
    if files.is_empty() { return None; }
    let total: usize = files.iter().map(|f| f.1.len()).sum();
    let mut o = format!("{total} lines in {} files\n", files.len());
    for (name, rows) in files.iter().take(25) {
        o.push_str(&format!("{name}\n"));
        push_capped(&mut o, &rows.iter().map(|r| format!("  {r}")).collect::<Vec<_>>(), 6);
    }
    if files.len() > 25 { o.push_str(&format!("... +{} files\n", files.len() - 25)); }
    Some(o)
}

fn is_comment(l: &str, ext: &str) -> bool {
    let t = l.trim_start();
    match ext {
        "py" | "sh" | "rb" | "yml" | "yaml" | "toml" | "pl" => t.starts_with("# ") || t == "#",
        "sql" | "lua" | "hs" => t.starts_with("-- "),
        _ => t.starts_with("// ") || t.starts_with("//") && t.len() <= 3 || t.starts_with("/*") || (t.starts_with("* ") || t == "*") || t.starts_with("*/") || t.starts_with("///"),
    }
}

/// Large file dump: blank runs collapsed, comment-only runs (3+) replaced by a count, long files capped.
pub fn file_dump(args: &[String], lines: &[String]) -> Option<String> {
    let file = positional(args).last().copied()?;
    let ext = file.rsplit('.').next().unwrap_or("");
    let src = ["rs", "py", "js", "ts", "tsx", "jsx", "go", "java", "c", "h", "cpp", "hpp", "cs", "rb", "sh", "kt", "swift", "php", "sql", "lua", "yml", "yaml", "toml"];
    if lines.len() < 60 || !src.contains(&ext) { return None; }
    let mut out: Vec<String> = vec![];
    let (mut blank, mut run): (bool, Vec<&String>) = (false, vec![]);
    let flush = |run: &mut Vec<&String>, out: &mut Vec<String>| {
        if run.len() >= 3 { out.push(format!("{}// ... {} comment lines", " ".repeat(run[0].len() - run[0].trim_start().len()), run.len())); }
        else { out.extend(run.iter().map(|s| s.to_string())); }
        run.clear();
    };
    for l in lines {
        if is_comment(l, ext) && !l.starts_with("#!") { run.push(l); continue; }
        flush(&mut run, &mut out);
        if l.trim().is_empty() { if !blank { out.push(String::new()); } blank = true; continue; }
        blank = false;
        out.push(l.trim_end().to_string());
    }
    flush(&mut run, &mut out);
    let n = out.len();
    if n > 200 {
        let tail: Vec<String> = out[n - 30..].to_vec();
        out.truncate(150);
        out.push(format!("... {} lines omitted ...", n - 180));
        out.extend(tail);
    }
    Some(out.join("\n") + "\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    fn l(s: &str) -> Vec<String> { s.lines().map(String::from).collect() }
    #[test]
    fn astgrep_groups_files() {
        let raw = "src/a.rs\n12│  let x = foo(1);\n40│  foo(2);\nsrc/b.rs\n3│  foo(3);\n";
        let o = astgrep(&l(raw)).unwrap();
        assert!(o.starts_with("3 lines in 2 files") && o.contains("src/a.rs\n  12: let x = foo(1);"));
        let inline = astgrep(&l("src/c.rs:7:5: foo(9)\nsrc/c.rs:9:1: foo(8)")).unwrap();
        assert!(inline.contains("2 lines in 1 files"));
    }
    #[test]
    fn dump_trims_blank_and_comment_runs() {
        let mut raw = String::from("#!/bin/sh\nfn_a() {\n");
        for i in 0..8 { raw.push_str(&format!("# explanation line {i}\n")); }
        for _ in 0..5 { raw.push_str("\n"); }
        for i in 0..60 { raw.push_str(&format!("echo {i}\n")); }
        raw.push_str("# one\n}\n");
        let lines = l(&raw);
        let o = file_dump(&["run.sh".into()], &lines).unwrap();
        assert!(o.contains("#!/bin/sh") && o.contains("// ... 8 comment lines") && !o.contains("explanation line 3") && o.contains("# one"));
        assert!(!o.contains("\n\n\n") && o.len() < raw.len());
    }
    #[test]
    fn dump_caps_long_files() {
        let raw: Vec<String> = (0..400).map(|i| format!("let v{i} = {i};")).collect();
        let o = file_dump(&["big.ts".into()], &raw).unwrap();
        assert!(o.contains("220 lines omitted") && o.contains("let v399") && !o.contains("let v200 "));
    }
    #[test]
    fn dump_skips_small_or_unknown() {
        assert!(file_dump(&["a.rs".into()], &l("fn main() {}")).is_none());
        assert!(file_dump(&["a.md".into()], &vec!["x".to_string(); 100]).is_none());
    }
}
