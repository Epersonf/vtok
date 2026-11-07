//! Shared helpers for the language-ecosystem views: command parsing, block collection, summaries.
use regex::Regex;
use std::collections::HashSet;

pub fn re(p: &str) -> Regex { Regex::new(p).expect("valid regex") }

pub struct Cmd { pub tool: String, pub args: Vec<String> }

const KNOWN: &[&str] = &["vitest", "jest", "tsc", "eslint", "prettier", "playwright", "next", "ng", "prisma", "biome", "oxlint", "vue-tsc"];

/// Resolve which tool a command line runs, tolerating `cd x &&`, env vars and runner prefixes
/// (npx, pnpm exec, bunx, python -m, uv run, bundle exec, php, vendor/bin/...).
pub fn parse(cmd: &str) -> Option<Cmd> {
    let seg = cmd.split("&&").last()?.split(';').last()?.split('|').next()?;
    let mut t: Vec<String> = seg.split_whitespace().map(|s| s.trim_matches(|c| c == '"' || c == '\'').to_string()).collect();
    loop {
        let f = t.first()?.clone();
        let env = f.split_once('=').map_or(false, |(k, _)| !k.is_empty() && k.chars().all(|c| c.is_alphanumeric() || c == '_'));
        if env || matches!(f.as_str(), "env" | "time" | "command" | "php" | "php8" | "php83" | "php84") { t.remove(0); continue; }
        if matches!(f.as_str(), "npx" | "bunx" | "pnpx") {
            t.remove(0);
            while t.first().map_or(false, |a| a.starts_with('-')) { t.remove(0); }
            continue;
        }
        if t.len() > 1 {
            let s = t[1].as_str();
            let wrap = match f.as_str() {
                "pnpm" | "npm" | "yarn" | "bun" => matches!(s, "exec" | "x" | "dlx"),
                "uv" | "poetry" | "pipenv" | "pdm" => s == "run",
                "bundle" => s == "exec",
                "python" | "python3" | "py" => s == "-m",
                _ => false,
            };
            if wrap { t.drain(0..2); continue; }
        }
        break;
    }
    let base = t[0].rsplit(['/', '\\']).next().unwrap_or("").to_lowercase();
    let tool = ["exe", "cmd", "bat", "phar", "ps1"].iter().find_map(|e| base.strip_suffix(&format!(".{e}"))).unwrap_or(&base).to_string();
    let mut args = t[1..].to_vec();
    if matches!(tool.as_str(), "pnpm" | "yarn" | "bun") && args.first().map_or(false, |a| KNOWN.contains(&a.as_str())) {
        let tool = args.remove(0);
        return Some(Cmd { tool, args });
    }
    Some(Cmd { tool, args })
}

/// Collect blocks that begin at `start` and end at `stop` (or the next start); blank and `drop` lines
/// skipped, blocks capped at `cap` lines, duplicate blocks (same first line) emitted once.
pub fn blocks(lines: &[String], start: &dyn Fn(&str) -> bool, stop: &dyn Fn(&str) -> bool, cap: usize, drop: &dyn Fn(&str) -> bool) -> Vec<String> {
    let (mut out, mut seen) = (vec![], HashSet::new());
    let mut cur: Option<Vec<String>> = None;
    let mut flush = |c: Option<Vec<String>>, out: &mut Vec<String>| {
        if let Some(mut c) = c {
            if !seen.insert(c[0].clone()) { return; }
            if c.len() > cap { c.truncate(cap); c.push("  ...".into()); }
            out.extend(c);
        }
    };
    for l in lines {
        if start(l) { flush(cur.take(), &mut out); cur = Some(vec![l.trim_end().to_string()]); }
        else if cur.is_some() {
            if stop(l) { flush(cur.take(), &mut out); }
            else if !l.trim().is_empty() && !drop(l) { cur.as_mut().unwrap().push(l.trim_end().to_string()); }
        }
    }
    flush(cur.take(), &mut out);
    out
}

/// Trimmed lines matching `r`, in order.
pub fn grep(lines: &[String], r: &str) -> Vec<String> {
    let r = re(r);
    lines.iter().filter(|l| r.is_match(l)).map(|l| l.trim().to_string()).collect()
}

pub fn has(lines: &[String], r: &str) -> bool { let r = re(r); lines.iter().any(|l| r.is_match(l)) }

/// Count repeats, keep first-seen order: three copies of "x" become "x (x3)".
pub fn collapse(items: Vec<String>) -> Vec<String> {
    let mut order: Vec<(String, usize)> = vec![];
    for i in items {
        match order.iter_mut().find(|(k, _)| *k == i) { Some(e) => e.1 += 1, None => order.push((i, 1)) }
    }
    order.into_iter().map(|(k, n)| if n > 1 { format!("{k} (x{n})") } else { k }).collect()
}

/// Cap a list at `n` entries, noting how many were dropped.
pub fn cap(mut v: Vec<String>, n: usize) -> Vec<String> {
    if v.len() > n { let extra = v.len() - n; v.truncate(n); v.push(format!("... +{extra} more")); }
    v
}

pub fn short(s: &str, n: usize) -> String {
    if s.chars().count() <= n { s.to_string() } else { format!("{}...", s.chars().take(n).collect::<String>()) }
}

/// "a x3, b x1" style tally, most frequent first.
pub fn tally(items: &[String]) -> String {
    let mut m: Vec<(String, usize)> = vec![];
    for i in items { match m.iter_mut().find(|(k, _)| k == i) { Some(e) => e.1 += 1, None => m.push((i.clone(), 1)) } }
    m.sort_by(|a, b| b.1.cmp(&a.1));
    m.iter().map(|(k, n)| format!("{k} x{n}")).collect::<Vec<_>>().join(", ")
}

pub fn finish(v: Vec<String>) -> Option<String> {
    if v.is_empty() { None } else { Some(v.join("\n") + "\n") }
}

#[cfg(test)]
pub mod t {
    pub fn lines(s: &str) -> Vec<String> { s.lines().map(|l| l.to_string()).collect() }
    /// Unwrap the view, assert it is newline-terminated and strictly smaller than the input.
    pub fn small(out: Option<String>, input: &[String]) -> String {
        let o = out.expect("view returned None");
        assert!(o.ends_with('\n'));
        assert!(o.len() < input.join("\n").len(), "compact ({}) not smaller than input ({})", o.len(), input.join("\n").len());
        o
    }
}
