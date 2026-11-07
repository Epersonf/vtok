//! git: subcommand dispatch and `status` (short and long format).
use super::util::*;

/// The subcommand, skipping `-C dir`, `-c k=v` and other global flags.
pub fn subcommand(args: &[String]) -> Option<(&str, Vec<&str>)> {
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-C" | "-c" | "--git-dir" | "--work-tree" => i += 2,
            a if a.starts_with('-') => i += 1,
            _ => return Some((args[i].as_str(), args[i + 1..].iter().map(|s| s.as_str()).collect())),
        }
    }
    None
}

pub fn view(args: &[String], lines: &[String]) -> Option<String> {
    let (sub, rest) = subcommand(args)?;
    match sub {
        "status" => status(lines),
        "add" | "commit" | "push" | "pull" | "fetch" | "checkout" | "switch" | "merge" | "rebase" | "cherry-pick" | "reset" | "restore" =>
            super::git_ops::view(sub, lines),
        "branch" | "stash" | "tag" | "remote" | "blame" | "show" => super::git_misc::view(sub, &rest, lines),
        _ => None,
    }
}

fn code_word(w: &str) -> &'static str {
    match w {
        "modified" => "M", "new file" => "A", "deleted" => "D", "renamed" => "R", "copied" => "C",
        "both modified" | "both added" | "deleted by us" | "deleted by them" | "added by us" | "added by them" | "both deleted" => "U",
        _ => "?",
    }
}

#[derive(Default)]
struct St { head: String, staged: Vec<String>, changed: Vec<String>, untracked: Vec<String>, conflict: Vec<String> }

pub fn status(lines: &[String]) -> Option<String> {
    let mut st = St::default();
    let mut section = "";
    for l in lines {
        if let Some(b) = l.strip_prefix("## ") { st.head = b.to_string(); continue; }
        if let Some(b) = l.strip_prefix("On branch ") { st.head = b.to_string(); continue; }
        if l.starts_with("HEAD detached") { st.head = l.to_string(); continue; }
        if l.starts_with("Your branch") { st.head.push_str(&format!(" ({})", l.trim_start_matches("Your branch is ").trim_end_matches('.'))); continue; }
        if l.starts_with("Changes to be committed") { section = "staged"; continue; }
        if l.starts_with("Changes not staged") { section = "changed"; continue; }
        if l.starts_with("Untracked files") { section = "untracked"; continue; }
        if l.starts_with("Unmerged paths") { section = "conflict"; continue; }
        if l.starts_with("nothing to commit") { st.head.push_str(" clean"); continue; }
        if let Some(t) = l.strip_prefix('\t') {
            let (code, name) = match t.split_once(':') {
                Some((w, n)) => (code_word(w.trim()), n.trim().to_string()),
                None => ("?", t.trim().to_string()),
            };
            let entry = if section == "untracked" { name } else { format!("{code} {name}") };
            match section { "staged" => st.staged.push(entry), "changed" => st.changed.push(entry), "untracked" => st.untracked.push(entry), _ => st.conflict.push(entry) }
            continue;
        }
        // short / porcelain: "XY path"
        let b = l.as_bytes();
        let code = |c: u8| b"MADRCU?! ".contains(&c);
        if b.len() > 3 && b[2] == b' ' && code(b[0]) && code(b[1]) && !(b[0] == b' ' && b[1] == b' ') {
            let (x, y, name) = (b[0] as char, b[1] as char, l[3..].to_string());
            if x == '?' { st.untracked.push(name); }
            else if x == 'U' || y == 'U' || (x == 'A' && y == 'A') || (x == 'D' && y == 'D') { st.conflict.push(format!("U {name}")); }
            else { if x != ' ' { st.staged.push(format!("{x} {name}")); } if y != ' ' { st.changed.push(format!("{y} {name}")); } }
        }
    }
    if st.head.is_empty() && st.staged.is_empty() && st.changed.is_empty() && st.untracked.is_empty() && st.conflict.is_empty() { return None; }
    let mut out = format!("* {}\n", st.head.trim());
    for (label, v) in [("conflict", &st.conflict), ("staged", &st.staged), ("changed", &st.changed), ("untracked", &st.untracked)] {
        if !v.is_empty() { out.push_str(&format!("{label} ({}): {}\n", v.len(), joined(v, 15))); }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn l(s: &str) -> Vec<String> { s.lines().map(String::from).collect() }

    #[test]
    fn long_status() {
        let raw = "On branch main\nYour branch is ahead of 'origin/main' by 2 commits.\n  (use \"git push\" to publish your local commits)\n\nChanges to be committed:\n  (use \"git restore --staged <file>...\" to unstage)\n\tmodified:   src/a.rs\n\tnew file:   src/b.rs\n\nChanges not staged for commit:\n  (use \"git add <file>...\" to update what will be committed)\n  (use \"git restore <file>...\" to discard changes in working directory)\n\tmodified:   README.md\n\nUntracked files:\n  (use \"git add <file>...\" to include in what will be committed)\n\ttmp.txt\n\n";
        let lines = l(raw);
        let o = view(&["status".into()], &lines).unwrap();
        assert!(o.contains("* main (ahead of 'origin/main' by 2 commits)"));
        assert!(o.contains("staged (2): M src/a.rs A src/b.rs"));
        assert!(o.contains("untracked (1): tmp.txt"));
        assert!(!o.contains("use \"git"));
        assert!(o.len() < raw.len());
    }
    #[test]
    fn short_status_with_conflict() {
        let lines = l("## feat...origin/feat [ahead 1]\n M src/a.rs\nM  src/b.rs\n?? new.txt\nUU merge.rs");
        let o = status(&lines).unwrap();
        assert!(o.contains("conflict (1): U merge.rs"));
        assert!(o.contains("changed (1): M src/a.rs"));
        assert!(o.contains("staged (1): M src/b.rs"));
        assert!(o.contains("untracked (1): new.txt"));
    }
}
