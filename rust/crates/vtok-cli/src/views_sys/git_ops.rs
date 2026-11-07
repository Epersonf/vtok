//! git state-changing commands: add, commit, push, pull, fetch, checkout, merge, rebase.
use super::util::*;

pub fn view(sub: &str, lines: &[String]) -> Option<String> {
    let conflicts: Vec<String> = lines.iter().filter(|l| l.starts_with("CONFLICT") || l.starts_with("Automatic merge failed")
        || l.contains("could not apply") || l.starts_with("error:") || l.starts_with("fatal:") || l.contains("[rejected]")).cloned().collect();
    if !conflicts.is_empty() && sub != "commit" {
        let mut o = format!("FAILED git {sub}\n");
        push_capped(&mut o, &conflicts, 20);
        return Some(o);
    }
    match sub {
        "add" => Some(if lines.is_empty() { "ok\n".into() } else { return None }),
        "commit" => commit(lines),
        "push" => push(lines),
        "pull" | "merge" | "rebase" | "cherry-pick" => sync(sub, lines),
        "fetch" => {
            let new = lines.iter().filter(|l| l.contains("[new ")).count();
            let upd = lines.iter().filter(|l| l.contains(" -> ") && !l.contains("[new ")).count();
            Some(if new + upd == 0 { "ok fetched\n".into() } else { format!("ok fetched: {new} new, {upd} updated\n") })
        }
        "checkout" | "switch" => {
            let keep: Vec<String> = lines.iter().filter(|l| l.starts_with("Switched") || l.starts_with("Already on") || l.starts_with("Your branch")
                || l.starts_with("branch '") || l.starts_with("Previous HEAD") || l.starts_with("HEAD is now")).cloned().collect();
            let dirty = lines.iter().filter(|l| l.len() > 2 && l.as_bytes()[1] == b'\t').count();
            if keep.is_empty() { return None; }
            let mut o = keep.join("\n");
            if dirty > 0 { o.push_str(&format!("\n{dirty} local changes carried")); }
            Some(o + "\n")
        }
        "reset" | "restore" => {
            let n = lines.iter().filter(|l| l.contains('\t') || l.starts_with("Unstaged")).count();
            Some(if n == 0 && lines.is_empty() { "ok\n".into() } else if n > 0 { format!("ok ({n} paths)\n") } else { return None })
        }
        _ => None,
    }
}

fn commit(lines: &[String]) -> Option<String> {
    let head = lines.iter().find(|l| l.starts_with('[') && l.contains(']'))?;
    let stat = lines.iter().find(|l| l.contains("changed")).map(|l| l.trim().to_string()).unwrap_or_default();
    let hooks = lines.iter().filter(|l| l.contains("Passed") || l.contains("Failed") || l.contains("Skipped")).count();
    let mut o = format!("ok {head}\n");
    if !stat.is_empty() { o.push_str(&format!("{stat}\n")); }
    if hooks > 0 { o.push_str(&format!("({hooks} hook lines)\n")); }
    Some(o)
}

fn push(lines: &[String]) -> Option<String> {
    let refs: Vec<String> = lines.iter().filter(|l| l.contains(" -> ")).map(|l| l.trim().to_string()).collect();
    let urls: Vec<String> = lines.iter().filter(|l| l.starts_with("remote:") && l.contains("http")).map(|l| l.trim_start_matches("remote:").trim().to_string()).collect();
    if refs.is_empty() && !lines.iter().any(|l| l.contains("up-to-date")) { return None; }
    let mut o = if refs.is_empty() { "ok up-to-date\n".to_string() } else { format!("ok pushed\n{}\n", refs.join("\n")) };
    for u in urls { o.push_str(&format!("{u}\n")); }
    Some(o)
}

fn sync(sub: &str, lines: &[String]) -> Option<String> {
    if lines.iter().any(|l| l.contains("Already up to date") || l.contains("Already up-to-date")) { return Some("ok up to date\n".into()); }
    let files: Vec<String> = lines.iter().filter(|l| l.starts_with(' ') && l.contains(" | ")).map(|l| l.split('|').next().unwrap_or("").trim().to_string()).collect();
    let stat = lines.iter().find(|l| l.contains("changed")).map(|l| l.trim().to_string());
    let done = lines.iter().find(|l| l.starts_with("Successfully rebased") || l.starts_with("Fast-forward") || l.starts_with("Merge made")).cloned();
    if files.is_empty() && stat.is_none() && done.is_none() { return None; }
    let mut o = format!("ok {sub}");
    if let Some(d) = done { o.push_str(&format!(": {d}")); }
    o.push('\n');
    if let Some(s) = stat { o.push_str(&format!("{s}\n")); }
    if !files.is_empty() { o.push_str(&format!("files: {}\n", joined(&files, 12))); }
    Some(o)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn l(s: &str) -> Vec<String> { s.lines().map(String::from).collect() }
    #[test]
    fn commit_push() {
        let c = l("[main 3f2a1bc] fix parser\n 2 files changed, 10 insertions(+), 3 deletions(-)\n create mode 100644 a.rs");
        let o = view("commit", &c).unwrap();
        assert!(o.contains("[main 3f2a1bc] fix parser") && o.contains("2 files changed") && !o.contains("create mode"));
        let p = l("Enumerating objects: 5, done.\nCounting objects: 100% (5/5), done.\nWriting objects: 100% (3/3), 300 bytes | 300.00 KiB/s, done.\nremote: Create a pull request for 'x' on GitHub by visiting:\nremote:      https://github.com/o/r/pull/new/x\nTo github.com:o/r.git\n   abc123..def456  main -> main");
        let o = view("push", &p).unwrap();
        assert!(o.contains("main -> main") && o.contains("https://github.com") && !o.contains("Enumerating"));
        assert!(o.len() < p.join("\n").len());
    }
    #[test]
    fn pull_merge_conflict() {
        let c = l("Auto-merging a.rs\nCONFLICT (content): Merge conflict in a.rs\nAutomatic merge failed; fix conflicts and then commit the result.");
        let o = view("merge", &c).unwrap();
        assert!(o.starts_with("FAILED git merge") && o.contains("Merge conflict in a.rs") && !o.contains("Auto-merging"));
        let p = l("remote: Counting objects: 3, done.\nFrom github.com:o/r\n   a1..b2  main       -> origin/main\nUpdating a1..b2\nFast-forward\n src/x.rs | 4 ++--\n src/y.rs | 2 +-\n 2 files changed, 3 insertions(+), 3 deletions(-)");
        let o = view("pull", &p).unwrap();
        assert!(o.contains("files: src/x.rs src/y.rs") && !o.contains("remote:"));
        assert_eq!(view("pull", &l("Already up to date.")).unwrap(), "ok up to date\n");
    }
    #[test]
    fn fetch_checkout_add() {
        let f = l("From github.com:o/r\n * [new branch]      feat -> origin/feat\n   a1..b2  main -> origin/main");
        assert_eq!(view("fetch", &f).unwrap(), "ok fetched: 1 new, 1 updated\n");
        let c = l("M\tsrc/a.rs\nSwitched to branch 'dev'\nYour branch is up to date with 'origin/dev'.");
        let o = view("checkout", &c).unwrap();
        assert!(o.contains("Switched to branch 'dev'") && o.contains("1 local changes") && !o.contains("M\t"));
        assert_eq!(view("add", &[]).unwrap(), "ok\n");
    }
}
