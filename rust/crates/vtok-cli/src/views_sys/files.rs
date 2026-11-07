//! ls (plain and -l) and tree.
use super::util::*;

const NOISE: [&str; 7] = ["node_modules", ".git", "target", "__pycache__", ".venv", "dist", ".next"];

pub fn ls(lines: &[String]) -> Option<String> {
    let long = lines.iter().any(|l| l.len() > 10 && matches!(l.as_bytes()[0], b'd' | b'-' | b'l') && l.as_bytes()[1..10].iter().all(|c| b"rwxsStT-".contains(c)));
    let (mut dirs, mut files): (Vec<String>, Vec<String>) = (vec![], vec![]);
    if long {
        for l in lines {
            let f: Vec<&str> = l.split_whitespace().collect();
            if f.len() < 9 || l.starts_with("total") { continue; }
            // name starts after the 8th field (perms links owner group size mon day time)
            let name = f[8..].join(" ");
            if name == "." || name == ".." { continue; }
            match l.as_bytes()[0] {
                b'd' => dirs.push(format!("{name}/")),
                _ => files.push(format!("{name} {}", f[4].parse::<u64>().map(human).unwrap_or_default())),
            }
        }
    } else {
        for name in lines.iter().flat_map(|l| l.split_whitespace()) {
            if name.ends_with('/') { dirs.push(name.to_string()) } else { files.push(name.to_string()) }
        }
    }
    if dirs.is_empty() && files.is_empty() { return None; }
    let mut o = format!("{} entries\n", dirs.len() + files.len());
    if !dirs.is_empty() { o.push_str(&format!("dirs: {}\n", joined(&dirs, 40))); }
    if !files.is_empty() { o.push_str(&format!("files: {}\n", joined(&files, 60))); }
    Some(o)
}

pub fn tree(lines: &[String]) -> Option<String> {
    if !lines.iter().any(|l| l.contains("── ")) { return None; }
    let depth = |l: &str| l.find("── ").map(|i| l[..i].chars().count() / 4).unwrap_or(0);
    let (mut o, mut skip_below, mut hidden, mut shown) = (String::new(), None::<usize>, 0, 0);
    for l in lines {
        if l.contains(" director") && (l.contains(" file") || l.ends_with("directories")) { o.push_str(&format!("{}\n", l.trim())); continue; }
        let d = depth(l);
        if let Some(s) = skip_below { if d > s && l.contains("── ") { hidden += 1; continue; } skip_below = None; }
        if shown >= 150 { hidden += 1; continue; }
        let name = l.rsplit("── ").next().unwrap_or("").trim();
        if l.contains("── ") && NOISE.contains(&name) { skip_below = Some(d); o.push_str(&format!("{l}  (collapsed)\n")); shown += 1; continue; }
        o.push_str(l); o.push('\n'); shown += 1;
    }
    if hidden > 0 { o.push_str(&format!("... {hidden} entries hidden\n")); }
    Some(o)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn l(s: &str) -> Vec<String> { s.lines().map(String::from).collect() }
    #[test]
    fn ls_long_groups_and_humanizes() {
        let raw = "total 24\ndrwxr-xr-x  5 ana staff   160 Mar  1 10:00 .\ndrwxr-xr-x  9 ana staff   288 Mar  1 10:00 ..\ndrwxr-xr-x  3 ana staff    96 Mar  1 10:00 src\n-rw-r--r--  1 ana staff  2048 Mar  1 10:00 Cargo.toml\n-rw-r--r--  1 ana staff 1500000 Mar  1 10:00 big file.bin";
        let lines = l(raw);
        let o = ls(&lines).unwrap();
        assert!(o.contains("3 entries") && o.contains("dirs: src/") && o.contains("Cargo.toml 2.0K") && o.contains("big file.bin 1.4M"));
        assert!(!o.contains("staff") && !o.contains("Mar") && o.len() < raw.len());
    }
    #[test]
    fn ls_plain_counts() {
        let o = ls(&l("a.rs\nb.rs\nsrc/")).unwrap();
        assert!(o.starts_with("3 entries") && o.contains("dirs: src/"));
    }
    #[test]
    fn tree_collapses_noise() {
        let raw = ".\n├── Cargo.toml\n├── node_modules\n│   ├── a\n│   │   └── index.js\n│   └── b\n└── src\n    └── main.rs\n\n3 directories, 4 files";
        let lines = l(raw);
        let o = tree(&lines).unwrap();
        assert!(o.contains("node_modules  (collapsed)") && o.contains("main.rs") && o.contains("3 directories, 4 files"));
        assert!(!o.contains("index.js") && o.contains("hidden"));
    }
}
