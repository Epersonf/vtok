//! Reading cargo output (build/check/clippy/test/nextest/bench): diagnostics, libtest results, nextest verdicts.
use super::Groups;
use regex::Regex;

#[derive(Default)]
pub(super) struct Parsed {
    pub errors: Vec<String>,
    pub warns: Groups,
    /// `error: could not compile` / `test failed` lines
    pub tail: Vec<String>,
    /// failing tests with their output (libtest `---- x stdout ----` and nextest FAIL blocks)
    pub blocks: Vec<Vec<String>>,
    pub names: Vec<String>,
    pub bench: Vec<String>,
    pub passed: usize,
    pub failed: usize,
    pub ignored: usize,
    pub suites: usize,
    pub compiled: usize,
    pub finished: Option<String>,
    pub nx_sum: Option<String>,
    pub nx_failed: bool,
    /// lines no rule explains: program output, `--nocapture` prints
    pub other: usize,
    pub known: bool,
}

const NOISE_VERBS: [&str; 13] = ["Downloading ", "Downloaded ", "Locking ", "Updating ", "Adding ", "Blocking ", "Unpacking ", "Fresh ", "Fixed ", "Installing ", "Installed ", "Packaging ", "Generated "];

/// A rustc diagnostic's snippet lines: ` --> loc`, `  |`, `3 |  code`, `  = note: ...`, `...`.
fn is_snippet(l: &str) -> bool {
    let t = l.trim_start();
    l.starts_with("...") || t.starts_with("-->") || t.starts_with('|') || t.starts_with('=') || t.starts_with(':') || t.starts_with(|c: char| c.is_ascii_digit() || matches!(c, '^' | '~'))
}

/// Lines cargo, libtest or nextest print around the results: blank, rulers, notes, status verbs.
fn is_chatter(t: &str) -> bool {
    t.is_empty() || t.chars().all(|c| c == '-') || t.starts_with("note:") || t.starts_with("help:") || t == "failures:"
        || NOISE_VERBS.iter().any(|v| t.starts_with(v))
}

impl Parsed {
    pub fn parse(lines: &[String]) -> Parsed {
        let diag = Regex::new(r"^(warning|error)(\[\w+\])?: (.+)$").unwrap();
        let res = Regex::new(r"^test result: \w+\. (\d+) passed; (\d+) failed; (\d+) ignored").unwrap();
        let nx = Regex::new(r"^\s*(?:TRY \d+ )?(PASS|FAIL|SLOW|SKIP|LEAK|TIMEOUT|SIGABRT|SIGSEGV|START|RETRY|CANCEL)\s+\[").unwrap();
        let mut p = Parsed::default();
        let mut i = 0;
        while i < lines.len() {
            let l = lines[i].as_str();
            let t = l.trim_start();
            i += 1;
            if let Some(c) = diag.captures(l) {
                p.known = true;
                let msg = &c[3];
                if msg.starts_with("could not compile") || msg.starts_with("aborting") || msg.starts_with("test failed") {
                    p.tail.push(l.to_string());
                    continue;
                }
                let skip = (msg.starts_with('`') && msg.contains(" generated ")) || msg.starts_with("unused manifest") || msg.starts_with("build failed, waiting");
                let loc = lines[i..].iter().take(6).find_map(|x| x.trim_start().strip_prefix("--> ")).unwrap_or("");
                while i < lines.len() && is_snippet(&lines[i]) { i += 1; }
                if skip { continue; }
                let code = c.get(2).map_or("", |m| m.as_str());
                if &c[1] == "error" {
                    p.errors.push(if loc.is_empty() { format!("error{code}: {msg}") } else { format!("error{code}: {msg} @ {loc}") });
                } else {
                    p.warns.add(&format!("warning{code}: {msg}"), loc);
                }
            } else if let Some(c) = res.captures(t) {
                p.known = true;
                p.suites += 1;
                let n = |k: usize| c[k].parse::<usize>().unwrap_or(0);
                (p.passed, p.failed, p.ignored) = (p.passed + n(1), p.failed + n(2), p.ignored + n(3));
            } else if t.starts_with("Compiling ") || t.starts_with("Checking ") || t.starts_with("Documenting ") {
                p.known = true;
                p.compiled += 1;
            } else if t.starts_with("Finished ") {
                p.known = true;
                p.finished = Some(t.to_string());
            } else if t.starts_with("Running ") || t.starts_with("Doc-tests") || t.starts_with("running ") || t.starts_with("Starting ") {
                p.known = true;
            } else if t.starts_with("Summary [") {
                p.known = true;
                p.nx_sum = Some(t.to_string());
            } else if let Some(c) = nx.captures(l) {
                p.known = true;
                if matches!(&c[1], "FAIL" | "TIMEOUT" | "SIGABRT" | "SIGSEGV" | "LEAK") {
                    p.nx_failed = true;
                    let mut b = vec![l.trim().to_string()];
                    while i < lines.len() && !nx.is_match(&lines[i]) && !lines[i].starts_with("-----") && !lines[i].trim_start().starts_with("Summary") && b.len() < 14 {
                        if !lines[i].trim().is_empty() { b.push(lines[i].trim_end().to_string()); }
                        i += 1;
                    }
                    if !p.blocks.iter().any(|x| x[0] == b[0]) { p.blocks.push(b); }
                }
            } else if t.starts_with("test ") && t.ends_with("... FAILED") {
                p.known = true;
                p.names.push(t[5..t.len() - 11].to_string());
            } else if t.starts_with("test ") && t.contains(" ... ") && !t.contains(" bench: ") {
                p.known = true;
            } else if t.starts_with("---- ") && t.ends_with(" ----") {
                p.known = true;
                let mut b = vec![t.to_string()];
                while i < lines.len() {
                    let n = lines[i].trim_end();
                    if n.starts_with("---- ") || n == "failures:" || n.starts_with("test result:") { break; }
                    if !n.is_empty() && !n.starts_with("note: run with `RUST_BACKTRACE") { b.push(format!("  {}", n.trim_start())); }
                    i += 1;
                }
                p.blocks.push(b);
            } else if t.contains(" bench: ") {
                p.bench.push(t.to_string());
            } else if t.contains("time:") && t.contains('[') {
                let name = if t.starts_with("time:") && i >= 2 { lines[i - 2].trim() } else { "" };
                p.bench.push(format!("{name} {t}").trim().to_string());
            } else if !is_chatter(t) {
                p.other += 1;
            }
        }
        p
    }

    pub fn green(&self) -> bool { self.errors.is_empty() && self.blocks.is_empty() && self.names.is_empty() && self.failed == 0 && self.tail.is_empty() && !self.nx_failed }
}
