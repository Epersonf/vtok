//! git read-only listings: branch, stash, tag, remote, blame, show --stat.
use super::util::*;

pub fn view(sub: &str, rest: &[&str], lines: &[String]) -> Option<String> {
    match sub {
        "branch" => branch(lines),
        "stash" => stash(rest, lines),
        "tag" => Some(format!("{} tags: {}\n", lines.len(), joined(&lines.iter().rev().take(15).cloned().collect::<Vec<_>>(), 15))).filter(|_| !lines.is_empty() && !rest.iter().any(|a| *a == "-a" || a.starts_with("-m"))),
        "remote" => remote(lines),
        "blame" => blame(lines),
        "show" => show(lines),
        _ => None,
    }
}

fn branch(lines: &[String]) -> Option<String> {
    let mut cur = String::new();
    let mut others = vec![];
    for l in lines {
        let t = l.trim();
        let (is_cur, t) = match t.strip_prefix("* ") { Some(r) => (true, r), None => (false, t) };
        let mut w = t.split_whitespace();
        let name = w.next()?.to_string();
        // -vv: keep tracking info in [..]
        let track = t.find('[').and_then(|a| t[a..].find(']').map(|b| t[a..a + b + 1].to_string())).unwrap_or_default();
        let e = if track.is_empty() { name } else { format!("{name}{track}") };
        if is_cur { cur = e } else { others.push(e) }
    }
    Some(format!("* {cur}\n{} others: {}\n", others.len(), joined(&others, 20)))
}

fn stash(rest: &[&str], lines: &[String]) -> Option<String> {
    match rest.first().copied() {
        Some("list") => {
            let items: Vec<String> = lines.iter().map(|l| l.replacen("stash@{", "", 1).replacen("}: ", " ", 1).replace("WIP on ", "")).collect();
            Some(format!("{} stashes\n{}\n", items.len(), items.join("\n")))
        }
        None | Some("push") | Some("save") if lines.iter().any(|l| l.starts_with("Saved working")) => Some("ok stashed\n".into()),
        Some("pop") | Some("apply") => {
            let drop = lines.iter().find(|l| l.starts_with("Dropped")).map(|l| format!(" ({l})")).unwrap_or_default();
            let st = super::git::status(lines).unwrap_or_default();
            Some(format!("ok {}{drop}\n{st}", rest[0]))
        }
        _ => None,
    }
}

fn remote(lines: &[String]) -> Option<String> {
    let mut seen: Vec<String> = vec![];
    for l in lines {
        let mut p = l.split_whitespace();
        let (n, u) = (p.next()?, p.next()?);
        let e = format!("{n} {u}");
        if !seen.contains(&e) { seen.push(e); }
    }
    Some(seen.join("\n") + "\n")
}

fn blame(lines: &[String]) -> Option<String> {
    let re = regex::Regex::new(r"^\^?([0-9a-f]{7,40})\s+(?:\S+\s+)?\((.+?)\s+\d{4}-\d\d-\d\d[^)]*?\s(\d+)\)\s?(.*)$").ok()?;
    let (mut o, mut last) = (String::new(), String::new());
    for l in lines {
        let c = re.captures(l)?;
        let id = c[1].chars().take(7).collect::<String>();
        if id != last { o.push_str(&format!("{id} {}\n", c[2].trim())); last = id; }
        o.push_str(&format!("  {}) {}\n", &c[3], &c[4]));
    }
    Some(o)
}

fn show(lines: &[String]) -> Option<String> {
    if lines.iter().any(|l| l.starts_with("diff --git")) || !lines.first()?.starts_with("commit ") { return None; }
    let hash: String = lines[0][7..].chars().take(7).collect();
    let author = lines.iter().find(|l| l.starts_with("Author:")).map(|l| l[7..].trim().split('<').next().unwrap_or("").trim().to_string()).unwrap_or_default();
    let date = lines.iter().find(|l| l.starts_with("Date:")).map(|l| l[5..].trim().to_string()).unwrap_or_default();
    let msg: Vec<&str> = lines.iter().filter(|l| l.starts_with("    ")).map(|l| l.trim()).collect();
    let stats: Vec<String> = lines.iter().filter(|l| l.contains(" | ") || l.contains("changed")).map(|l| l.trim().to_string()).collect();
    let mut o = format!("{hash} {} ({author}, {date})\n", msg.first().unwrap_or(&""));
    push_capped(&mut o, &stats, 25);
    Some(o)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn l(s: &str) -> Vec<String> { s.lines().map(String::from).collect() }
    #[test]
    fn branches_and_stash() {
        let b = l("  dev\n* main\n  feat/a\n  feat/b\n  remotes/origin/HEAD -> origin/main");
        let o = view("branch", &[], &b).unwrap();
        assert!(o.starts_with("* main\n4 others: dev feat/a feat/b remotes/origin/HEAD"));
        let s = l("stash@{0}: WIP on main: abc1234 wip parser\nstash@{1}: On dev: experiment");
        let o = view("stash", &["list"], &s).unwrap();
        assert!(o.contains("2 stashes") && o.contains("0 main: abc1234") && !o.contains("stash@"));
        assert_eq!(view("stash", &[], &l("Saved working directory and index state WIP on main: a b")).unwrap(), "ok stashed\n");
    }
    #[test]
    fn remote_tag_blame_show() {
        let r = l("origin\thttps://x/y.git (fetch)\norigin\thttps://x/y.git (push)");
        assert_eq!(view("remote", &["-v"], &r).unwrap(), "origin https://x/y.git\n");
        let t = view("tag", &[], &l("v1.0\nv1.1")).unwrap();
        assert!(t.starts_with("2 tags"));
        let b = l("abc12345 (Ana Silva 2024-03-01 10:00:00 +0000 1) fn main() {\nabc12345 (Ana Silva 2024-03-01 10:00:00 +0000 2)     run();\n0fe98765 (Bob 2024-03-02 11:00:00 +0000 3) }");
        let o = view("blame", &[], &b).unwrap();
        assert_eq!(o.matches("abc1234 Ana Silva").count(), 1);
        assert!(o.contains("  2)     run();") && !o.contains("+0000"));
        let s = l("commit 1234567890abcdef\nAuthor: Ana <a@x.io>\nDate:   Mon Mar 4 2024\n\n    fix bug\n\n a.rs | 4 ++--\n 1 file changed, 2 insertions(+), 2 deletions(-)");
        let o = view("show", &["--stat"], &s).unwrap();
        assert!(o.starts_with("1234567 fix bug (Ana, Mon Mar 4 2024)") && o.contains("a.rs | 4") && !o.contains("<a@x.io>"));
    }
}
