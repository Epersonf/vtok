//! Per-tool views for `vtok run`, chosen by the shape of the output (so they work through the hook, the MCP tool and
//! plain `vtok run` alike): compiler/linter diagnostics grouped by rule and file (tsc, ruff, flake8, mypy, eslint
//! compact) and the long `git status` grouped by state. Each returns None when the output is not that shape.
use crate::diag::{render, D};

struct Diag { file: String, line: String, code: String, msg: String }

fn split_drive(l: &str) -> (&str, &str) {
    let b = l.as_bytes();
    if b.len() > 3 && b[1] == b':' && (b[2] == b'\\' || b[2] == b'/') && b[0].is_ascii_alphabetic() { l.split_at(2) } else { ("", l) }
}

fn is_code(w: &str) -> bool {
    w.len() >= 2 && w.len() <= 12 && w.chars().next().map_or(false, |c| c.is_ascii_uppercase()) && w.chars().any(|c| c.is_ascii_digit()) && w.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn parse_diag(l: &str) -> Option<Diag> {
    let l = l.trim();
    // tsc: path(12,5): error TS2322: message
    if let Some(i) = l.find("): error TS").or_else(|| l.find("): warning TS")) {
        let (path, pos) = l[..i].rsplit_once('(')?;
        let rest = &l[i + 3..];
        let (kind_code, msg) = rest.split_once(": ")?;
        let code = kind_code.split_whitespace().nth(1)?.to_string();
        return Some(Diag { file: path.into(), line: pos.split(',').next()?.into(), code, msg: msg.into() });
    }
    // path:line[:col]: CODE message   |   path:line[:col]: error: message  [code]
    let (drive, body) = split_drive(l);
    let mut it = body.splitn(4, ':');
    let file = it.next()?;
    let line = it.next()?;
    if file.is_empty() || !file.contains('.') || file.contains(' ') || line.is_empty() || !line.bytes().all(|b| b.is_ascii_digit()) { return None; }
    let mut rest = it.next()?.to_string();
    if let Some(r4) = it.next() { if rest.bytes().all(|b| b.is_ascii_digit()) && !rest.is_empty() { rest = r4.to_string(); } else { rest = format!("{rest}:{r4}"); } }
    let rest = rest.trim();
    let file = format!("{drive}{file}");
    for kind in ["error:", "warning:", "note:"] {
        if let Some(m) = rest.strip_prefix(kind) {
            if kind == "note:" { return None; }
            let m = m.trim();
            let (msg, code) = match (m.rfind('['), m.ends_with(']')) { (Some(i), true) => (m[..i].trim(), m[i + 1..m.len() - 1].to_string()), _ => (m, kind.trim_end_matches(':').to_string()) };
            return Some(Diag { file, line: line.into(), code, msg: msg.into() });
        }
    }
    let (code, msg) = rest.split_once(' ')?;
    if !is_code(code.trim_end_matches(':')) { return None; }
    Some(Diag { file, line: line.into(), code: code.trim_end_matches(':').into(), msg: msg.trim().into() })
}

/// Diagnostics grouped by (rule, message): one header with a count, then `file lines` rows.
pub fn diagnostics(lines: &[String]) -> Option<String> {
    let ds: Vec<Diag> = lines.iter().filter_map(|l| parse_diag(l)).collect();
    let nonempty = lines.iter().filter(|l| !l.trim().is_empty()).count();
    if ds.len() < 6 || ds.len() * 2 < nonempty { return None; }
    Some(render_diags(&ds, lines))
}

/// The grouped view of already parsed diagnostics (see `crate::diag`); `lines` supplies a trailing summary line if the
/// tool printed one.
fn render_diags(ds: &[Diag], lines: &[String]) -> String {
    let all: Vec<D> = ds.iter().map(|d| D::new(&d.file, d.line.parse().unwrap_or(0), 0, &d.code, &d.msg)).collect();
    let summary = lines.iter().rev().find(|l| parse_diag(l).is_none() && l.chars().any(|c| c.is_ascii_digit()) && !l.trim().is_empty()).map(|s| s.trim().to_string());
    render(&all, ("diagnostic", "diagnostics"), summary).join("\n") + "\n"
}

/// Long-format `git status`: branch line, then one row per state with the paths.
pub fn git_status(lines: &[String]) -> Option<String> {
    let first = lines.first()?;
    if !(first.starts_with("On branch ") || first.starts_with("HEAD detached")) { return None; }
    let mut out = vec![first.replacen("On branch ", "", 1)];
    let mut foot: Vec<String> = vec![];
    let mut rows: Vec<(String, Vec<String>)> = vec![];
    for l in &lines[1..] {
        let t = l.trim();
        if t.is_empty() || t.starts_with('(') { continue; }
        if t.starts_with("Your branch") { out.push(t.trim_end_matches('.').replace("Your branch is ", "").replace("Your branch and ", "")); continue; }
        if t.ends_with(':') && !l.starts_with('\t') { let section = match t { "Changes to be committed:" => "staged", "Changes not staged for commit:" => "unstaged", "Untracked files:" => "untracked", "Unmerged paths:" => "conflict", _ => "other" }; rows.push((section.to_string(), vec![])); continue; }
        if t.starts_with("no changes added") || t.starts_with("nothing to commit") || t.starts_with("nothing added") { foot.push(t.into()); continue; }
        if l.starts_with('\t') { if let Some(r) = rows.last_mut() {
            let item = match t.split_once(":   ") { Some((k, p)) => format!("{} {}", match k { "modified" => "M", "new file" => "A", "deleted" => "D", "renamed" => "R", "both modified" => "UU", other => other }, p.trim()), None => t.to_string() };
            r.1.push(item);
        } }
    }
    for (s, items) in rows { if !items.is_empty() { out.push(format!("{s}({}): {}", items.len(), items.join("  "))); } }
    out.extend(foot);
    Some(out.join("
") + "
")
}

pub fn view(lines: &[String]) -> Option<String> {
    git_status(lines).or_else(|| diagnostics(lines)).or_else(|| crate::filters_tools::view(lines))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn l(s: &str) -> Vec<String> { s.lines().map(String::from).collect() }

    #[test]
    fn tsc_errors_group_by_code() {
        let mut s = String::new();
        for i in 1..=5 { s += &format!("src/a.ts({i},3): error TS2322: Type 'x' is not assignable to type 'y'.\n"); }
        s += "src/b.ts(9,1): error TS2304: Cannot find name 'q'.\nFound 6 errors in 2 files.\n";
        let v = diagnostics(&l(&s)).unwrap();
        assert!(v.starts_with("TS2322 x5: Type 'x' is not assignable to type 'y'.\nsrc/a.ts:1,2,3,4,5\n"), "{v}");
        assert!(v.contains("src/b.ts:9 TS2304 Cannot find name 'q'.") && v.ends_with("Found 6 errors in 2 files.\n"), "{v}");
    }

    #[test]
    fn ruff_mypy_and_windows_paths() {
        let mut s = String::new();
        for i in 1..=4 { s += &format!("pkg/m.py:{i}:1: E501 line too long\n"); }
        s += "C:\\w\\x.py:3:5: F401 'os' imported but unused\npkg/n.py:7: error: Incompatible types  [assignment]\n";
        let v = diagnostics(&l(&s)).unwrap();
        assert!(v.contains("E501 x4: line too long\npkg/m.py:1,2,3,4\n") && v.contains("C:/w/x.py:3 F401 'os' imported but unused"), "{v}");
        assert!(v.contains("pkg/n.py:7 assignment Incompatible types"), "{v}");
    }

    #[test]
    fn prose_is_not_diagnostics() {
        assert!(diagnostics(&l("a: b\nhello world\nsrc/x.rs:1: something\n")).is_none());
        assert!(diagnostics(&l("error: could not compile\n--> src/lib.rs:3:5\n")).is_none());
    }

    #[test]
    fn status_is_grouped() {
        let s = "On branch main\nYour branch is up to date with 'origin/main'.\n\nChanges not staged for commit:\n  (use \"git add <file>...\" to update)\n\tmodified:   a.rs\n\tdeleted:    b.rs\n\nUntracked files:\n  (use \"git add\")\n\tnew.txt\n\nno changes added to commit\n";
        let v = git_status(&l(s)).unwrap();
        assert_eq!(v, "main\nup to date with 'origin/main'\nunstaged(2): M a.rs  D b.rs\nuntracked(1): new.txt\nno changes added to commit\n");
    }
}
