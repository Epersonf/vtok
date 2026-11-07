//! tsc / vue-tsc / tsc -b, plain (`a.ts(1,14): error TS2322: ...`) or `--pretty` (ANSI already stripped, with code frames
//! and related-information blocks), watch-mode banners included. Every error is kept, grouped by (code, message,
//! elaboration): the message once, then the lines it happens on, see `crate::diag`.
use super::util::{finish, re};
use crate::diag::{plural, render, D};

fn is_frame(l: &str) -> bool {
    let t = l.trim_start();
    t.starts_with('~') || t.starts_with('^') || t.split_whitespace().next().map_or(false, |w| w.bytes().all(|b| b.is_ascii_digit()))
}

/// The `Errors  Files` table, watch banners, and what `npm run` / `yarn` / `bun run` wrap around a failing script.
fn is_table_or_banner(l: &str) -> bool {
    let t = l.trim();
    t.starts_with("Errors  Files") || t.starts_with("Version ") || t.starts_with('[') && t.contains("] ")
        || t.split_whitespace().count() == 2 && t.split_whitespace().next().map_or(false, |w| w.bytes().all(|b| b.is_ascii_digit()))
        || ["> ", "$ ", "npm ", "pnpm ", "yarn ", "bun ", "ELIFECYCLE", "error Command failed", "info Visit", "Exit status"].iter().any(|p| t.starts_with(p))
}

#[derive(PartialEq)]
enum St { Elaboration, Skip }

/// Diagnostics, the tool's own `Found N errors` line, how many lines were not understood.
struct Parsed { ds: Vec<D>, found: Option<String>, reported: Option<usize>, other: usize }

fn parse(lines: &[String]) -> Parsed {
    let plain = re(r"^(.+?)\((\d+),(\d+)\): (error|warning) (TS\d+): (.*)$");
    let pretty = re(r"^(.+?):(\d+):(\d+) - (error|warning) (TS\d+): (.*)$");
    let loose = re(r"^(?:(\S.*?): )?(error|warning) (TS\d+): (.*)$");
    let related = re(r"^\s+(\S.*?):(\d+):\d+ - (.*)$");
    let found = re(r"^(?:\[[^\]]*\]\s+)?(Found (\d+) errors?\b.*)$");
    let mut p = Parsed { ds: vec![], found: None, reported: None, other: 0 };
    let mut st = St::Skip;
    for l in lines {
        let hit = plain.captures(l).or_else(|| pretty.captures(l));
        if let Some(c) = hit {
            let mut d = D::new(&c[1], c[2].parse().unwrap_or(0), c[3].parse().unwrap_or(0), &c[5], c[6].trim());
            d.warn = &c[4] == "warning";
            p.ds.push(d);
            st = St::Elaboration;
        } else if let Some(c) = found.captures(l) {
            p.found = Some(c[1].to_string());
            p.reported = c[2].parse().ok();
            st = St::Skip;
        } else if let Some(c) = loose.captures(l).filter(|_| !l.starts_with(char::is_whitespace)) {
            let mut d = D::new(c.get(1).map_or("", |m| m.as_str()), 0, 0, &c[3], c[4].trim());
            d.warn = &c[2] == "warning";
            p.ds.push(d);
            st = St::Elaboration;
        } else if l.trim().is_empty() {
            st = St::Skip;
        } else if let (Some(c), Some(last)) = (related.captures(l), p.ds.last_mut()) {
            last.extra.push(format!("see {}:{}: {}", c[1].replace('\\', "/"), &c[2], c[3].trim()));
            st = St::Skip;
        } else if st == St::Elaboration && l.starts_with(' ') && !is_frame(l) {
            if let Some(last) = p.ds.last_mut() { last.extra.push(l.strip_prefix("  ").unwrap_or(l).trim_end().to_string()); }
        } else if !(is_frame(l) || is_table_or_banner(l)) {
            p.other += 1;
        }
    }
    p
}

pub fn tsc(lines: &[String]) -> Option<String> {
    let p = parse(lines);
    if p.ds.is_empty() { return p.found.and_then(|f| finish(vec![f])); }
    // the tool counted more errors than we could read: let the generic summariser see the raw text
    if p.reported.map_or(false, |n| n > p.ds.len()) { return None; }
    let mut out = render(&p.ds, ("error", "errors"), p.found);
    if p.other > 0 { let at = out.len() - 1; out.insert(at, format!("({} omitted)", plural(p.other, "other line", "other lines"))); }
    finish(out)
}
