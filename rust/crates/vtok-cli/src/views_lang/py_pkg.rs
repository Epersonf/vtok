//! pip, uv and sqlfluff: what was installed / changed, errors, violations per file.
use super::util::*;

pub fn pip(args: &[String], lines: &[String]) -> Option<String> {
    let sub = args.iter().find(|a| !a.starts_with('-')).map(|s| s.as_str()).unwrap_or("");
    match sub {
        "install" | "uninstall" | "download" => {
            let satisfied = grep(lines, r"^Requirement already satisfied").len();
            let mut out = grep(lines, r"^(Successfully (installed|uninstalled)|ERROR|WARNING|Found existing installation|Skipping|Would install)");
            if satisfied > 0 { out.insert(0, format!("{satisfied} requirement(s) already satisfied")); }
            if out.is_empty() { return None; }
            finish(cap(collapse(out), 20))
        }
        "list" => {
            let rows: Vec<String> = lines.iter().filter(|l| !l.starts_with("Package") && !l.starts_with("---") && !l.trim().is_empty())
                .filter_map(|l| { let c: Vec<&str> = l.split_whitespace().collect(); if c.len() >= 2 { Some(if c.len() >= 4 { format!("{} {} -> {}", c[0], c[1], c[2]) } else { format!("{}=={}", c[0], c[1]) }) } else { None } }).collect();
            if rows.is_empty() || !lines.iter().any(|l| l.starts_with("Package")) { return None; }
            finish(vec![format!("{} package(s): {}", rows.len(), rows.join(", "))])
        }
        _ => None,
    }
}

pub fn uv(args: &[String], lines: &[String]) -> Option<String> {
    let sub = args.iter().find(|a| !a.starts_with('-')).map(|s| s.as_str()).unwrap_or("");
    if !matches!(sub, "pip" | "sync" | "add" | "remove" | "lock" | "install" | "venv" | "run") { return None; }
    let changes: Vec<String> = lines.iter().filter(|l| re(r"^\s*[+\-~] \S+==").is_match(l)).map(|l| l.trim().to_string()).collect();
    let mut out = grep(lines, r"^(Resolved|Installed|Uninstalled|Audited|Prepared|Updated|Added|Removed|error|warning|Using|Creating|Built)\b");
    out.retain(|l| !l.starts_with("Using CPython") || sub == "venv");
    if out.is_empty() && changes.is_empty() { return None; }
    out.extend(cap(changes, 25));
    finish(out)
}

pub fn sqlfluff(lines: &[String]) -> Option<String> {
    let item = re(r"^L:\s*(\d+)\s*\|\s*P:\s*(\d+)\s*\|\s*(\w+)\s*\|\s*(.*)$");
    let (mut out, mut codes) = (vec![], vec![]);
    for l in lines {
        if let Some(c) = item.captures(l) {
            codes.push(c[3].to_string());
            out.push(format!("  L{}:{} {} {}", &c[1], &c[2], &c[3], short(&c[4], 140)));
        } else if let Some(i) = l.strip_prefix("== [").and_then(|r| r.split_once("] ")) {
            if i.1.starts_with("FAIL") { out.push(i.0.to_string()); }
        }
    }
    let tail = grep(lines, r"violations? found|fixable|^All Finished|Error|error");
    if out.is_empty() && tail.is_empty() { return None; }
    let mut v = cap(out, 80);
    if codes.len() > 5 { v.push(format!("by rule: {}", tally(&codes))); }
    v.extend(tail.into_iter().filter(|l| !l.starts_with("All Finished")));
    finish(v)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::util::t::*;
    fn a(s: &str) -> Vec<String> { s.split_whitespace().map(String::from).collect() }

    #[test]
    fn pip_install_collapses() {
        let mut s = String::new();
        for i in 0..8 { s += &format!("Requirement already satisfied: dep{i} in /venv/lib (1.{i})\n"); }
        s += "Collecting requests\n  Downloading requests-2.31.0-py3-none-any.whl (62 kB)\n     ---------------------------------------- 62.6/62.6 kB 1.2 MB/s eta 0:00:00\nInstalling collected packages: requests\nSuccessfully installed requests-2.31.0\n";
        let l = lines(&s);
        let o = small(pip(&a("install requests"), &l), &l);
        assert!(o.contains("8 requirement(s) already satisfied") && o.contains("Successfully installed requests-2.31.0"));
        assert!(!o.contains("Downloading") && !o.contains("dep3"));
    }

    #[test]
    fn pip_list_compacts() {
        let l = lines("Package    Version\n---------- -------\nrequests   2.31.0\nflask      3.0.0\n");
        assert!(pip(&a("list"), &l).unwrap().contains("2 package(s): requests==2.31.0, flask==3.0.0"));
    }

    #[test]
    fn uv_sync_changes() {
        let l = lines("Resolved 42 packages in 120ms\nPrepared 3 packages in 1.2s\nInstalled 3 packages in 20ms\n + anyio==4.3.0\n + httpx==0.27.0\n - old==1.0\n");
        let o = uv(&a("pip install httpx"), &l).unwrap();
        assert!(o.contains("Resolved 42 packages") && o.contains("+ httpx==0.27.0") && o.contains("- old==1.0"));
    }

    #[test]
    fn sqlfluff_lint_fail_files_only() {
        let s = "== [models/a.sql] FAIL\nL:   1 | P:   1 | LT01 | Unnecessary trailing whitespace. [layout.spacing]\nL:   4 | P:   5 | AM04 | Query produces an unknown number of result columns. [ambiguous.column_count]\n== [models/b.sql] PASS\n== [models/c.sql] PASS\n== [models/d.sql] PASS\nAll Finished 📜 🎉!\n";
        let l = lines(s);
        let o = small(sqlfluff(&l), &l);
        assert!(o.contains("models/a.sql") && o.contains("L4:5 AM04") && !o.contains("models/b.sql") && !o.contains("All Finished"));
    }
}
