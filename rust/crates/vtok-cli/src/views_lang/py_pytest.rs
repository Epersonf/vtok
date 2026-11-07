//! pytest, plain text, whatever the flags (-q/-v/-x, --tb=short|long|line|no, -r, xdist): the counts line, then one line
//! per failing or erroring test with `file:line name: message`. Parsing here; the lines are built in `py_pytest_fmt`.
use super::util::re;

/// One entry of the FAILURES / ERRORS sections.
#[derive(Default)]
pub(super) struct Block {
    pub title: String,
    pub e: Vec<String>,
    /// `path:line:` lines with the number of `E` lines seen before them
    locs: Vec<(usize, String, String)>,
    pub captured: bool,
}

impl Block {
    /// Where it failed: an `E File "x.py", line N` (syntax errors), else the frame line after the last `E` line (long
    /// format), else the last frame line before the first `E` line (short format).
    pub fn location(&self) -> Option<(String, String)> {
        if let Some(c) = self.e.first().and_then(|e| re(r#"^File "(.+)", line (\d+)"#).captures(e)) { return Some((c[1].to_string(), c[2].to_string())); }
        let n = self.e.len();
        let pick = self.locs.iter().find(|l| n > 0 && l.0 == n).or_else(|| self.locs.iter().rev().find(|l| l.0 == 0));
        pick.map(|l| (l.1.clone(), l.2.clone()))
    }
}

/// `FAILED nodeid - message` / `ERROR nodeid - message` of the short test summary.
pub(super) struct Entry { pub status: String, pub nodeid: String, pub msg: String }

#[derive(Default)]
pub(super) struct Report {
    pub counts: String,
    pub rootdir: String,
    pub blocks: Vec<Block>,
    pub entries: Vec<Entry>,
    /// (reason, count) from `SKIPPED [n] loc: reason`
    pub skipped: Vec<(String, usize)>,
    pub xfail: Vec<String>,
    pub xpass: Vec<String>,
    /// `stopping after 1 failures`, `Interrupted: 2 errors during collection`
    pub notes: Vec<String>,
}

#[derive(PartialEq, Clone, Copy)]
enum Sec { Head, Problems, Summary, Other }

/// Split `nodeid - message` at the first ` - ` outside brackets (parametrize ids may contain it).
pub(super) fn split_nodeid(s: &str) -> (String, String) {
    let mut depth = 0i32;
    for (i, c) in s.char_indices() {
        match c { '[' => depth += 1, ']' => depth -= 1, _ => {} }
        if depth <= 0 && s[i..].starts_with(" - ") { return (s[..i].to_string(), s[i + 3..].trim().to_string()); }
    }
    (s.trim().to_string(), String::new())
}

fn is_e(l: &str) -> bool { l == "E" || l.starts_with("E ") || l.starts_with("E\t") }

impl Report {
    /// None unless the text ends in pytest's `N failed, M passed in 1.2s` line.
    pub fn parse(lines: &[String]) -> Option<Report> {
        let cnt = re(r"^(?:\d+ [A-Za-z][A-Za-z ]*?(?:, \d+ [A-Za-z][A-Za-z ]*?)*|no tests ran) in \d+(?:\.\d+)?s(?: \(\d+:\d{2}:\d{2}\))?$");
        let head = re(r"^=+ (.+?) =+$");
        let block = re(r"^_+ (.+?) _+$");
        let loc = re(r"^(?P<p>\S.*?\.pyi?):(?P<n>\d+):(?P<rest>.*)$");
        let cap = re(r"^-+ Captured (?:stdout|stderr|log) \w+ -+$");
        let mut rep = Report::default();
        rep.counts = lines.iter().rev().find_map(|l| { let t = l.trim().trim_matches('=').trim(); cnt.is_match(t).then(|| t.to_string()) })?;
        let mut sec = Sec::Head;
        let mut cur: Option<Block> = None;
        let mut flat: Vec<Block> = vec![]; // --tb=line: no headers, `E` lines then one `path:line: message` line
        let mut run = Vec::<String>::new();
        let mut in_cap = false;
        for l in lines {
            if let Some(c) = head.captures(l) {
                rep.blocks.extend(cur.take());
                sec = match &c[1] { "ERRORS" | "FAILURES" => Sec::Problems, "short test summary info" => Sec::Summary, "test session starts" => Sec::Head, _ => Sec::Other };
                (run, in_cap) = (vec![], false);
            } else if l.starts_with("!!!") {
                let n = l.trim_matches(|c: char| c == '!' || c == ' ');
                if n.starts_with("stopping after") || n.starts_with("Interrupted") { rep.notes.push(n.to_string()); }
            } else if let Some(r) = l.strip_prefix("rootdir: ") {
                rep.rootdir = r.split(',').next().unwrap_or("").trim().replace('\\', "/");
            } else if sec == Sec::Summary {
                rep.summary_line(l);
            } else if sec == Sec::Problems {
                if let Some(c) = block.captures(l) {
                    rep.blocks.extend(cur.take());
                    cur = Some(Block { title: c[1].to_string(), ..Default::default() });
                    in_cap = false;
                } else if cap.is_match(l) {
                    in_cap = true;
                    if let Some(b) = cur.as_mut() { b.captured = true; }
                } else if in_cap {
                } else if let Some(b) = cur.as_mut() {
                    if is_e(l) { b.e.push(l[1..].trim().to_string()); }
                    else if !l.starts_with(char::is_whitespace) { if let Some(c) = loc.captures(l) { b.locs.push((b.e.len(), c["p"].to_string(), c["n"].to_string())); } }
                } else if is_e(l) {
                    run.push(l[1..].trim().to_string());
                } else if let Some(c) = loc.captures(l) {
                    let mut e = std::mem::take(&mut run);
                    if e.is_empty() { e.push(c["rest"].trim().to_string()); }
                    flat.push(Block { e, locs: vec![(0, c["p"].to_string(), c["n"].to_string())], ..Default::default() });
                }
            }
        }
        rep.blocks.extend(cur.take());
        rep.blocks.extend(flat);
        Some(rep)
    }

    fn summary_line(&mut self, l: &str) {
        let l = l.trim();
        let status = l.split_whitespace().next().unwrap_or("");
        let rest = l[status.len()..].trim_start();
        match status {
            "FAILED" | "ERROR" => { let (nodeid, msg) = split_nodeid(rest); self.entries.push(Entry { status: status.into(), nodeid: nodeid.replace('\\', "/"), msg }); }
            "SKIPPED" => {
                let n = rest.strip_prefix('[').and_then(|r| r.split_once(']')).and_then(|(n, _)| n.parse::<usize>().ok()).unwrap_or(1);
                let reason = rest.splitn(3, ": ").nth(1).map(str::trim).unwrap_or("").to_string();
                match self.skipped.iter_mut().find(|(r, _)| *r == reason) { Some(e) => e.1 += n, None => self.skipped.push((reason, n)) }
            }
            "XFAIL" => self.xfail.push(split_nodeid(rest).0),
            "XPASS" => self.xpass.push(split_nodeid(rest).0),
            _ => {}
        }
    }
}
