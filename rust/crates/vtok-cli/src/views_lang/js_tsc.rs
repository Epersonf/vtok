//! prettier --check: every offending file, nothing else (tsc has its own module, `ts_tsc`).
use super::util::*;

pub fn prettier(lines: &[String]) -> Option<String> {
    let ok = has(lines, r"All matched files use Prettier code style");
    let warn = re(r"^\[warn\] (.+)$");
    let mut files = vec![];
    let mut out = vec![];
    for l in lines {
        if let Some(c) = warn.captures(l) {
            let m = c[1].trim().to_string();
            if m.starts_with("Code style issues") { out.push(m); } else { files.push(m); }
        } else if l.starts_with("[error]") { out.push(l.trim().to_string()); }
    }
    if ok { return finish(vec!["prettier: all files formatted".into()]); }
    if files.is_empty() && out.is_empty() { return None; }
    let n = files.len();
    let mut v = vec![format!("prettier: {n} file(s) need formatting")];
    v.extend(cap(files, 30));
    v.extend(out);
    finish(v)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::util::t::*;

    #[test]
    fn prettier_lists_files() {
        let l = lines("Checking formatting...\n[warn] src/a.ts\n[warn] src/b.ts\n[warn] Code style issues found in 2 files. Run Prettier with --write to fix.\n");
        let o = prettier(&l).unwrap();
        assert!(o.contains("2 file(s)") && o.contains("src/b.ts") && o.contains("Code style issues found in 2 files"));
        assert!(!o.contains("Checking formatting"));
        let ok = prettier(&lines("Checking formatting...\nAll matched files use Prettier code style!")).unwrap();
        assert!(ok.contains("all files formatted"));
    }
}
