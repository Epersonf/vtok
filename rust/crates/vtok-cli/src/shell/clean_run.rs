//! Runs that went fine need only their summary: noise lines, passing-test lines and per-suite counters are folded.
use super::clean::cap;
use super::score::score;
use super::summary::{collapse, render};
use std::collections::BTreeSet;
use vtok_api::ntok;

pub(super) const CLEAN_BUDGET: usize = 150;

/// Build-tool chatter that never decides anything (kept in the stored full output).
fn is_noise(l: &str) -> bool {
    let t = l.trim_start();
    ["Compiling ", "Finished ", "Running ", "Downloaded ", "Downloading ", "Doc-tests ", "Blocking waiting", "Updating ", "Locking "].iter().any(|p| t.starts_with(p))
        || t.starts_with("running ") && t.ends_with(" tests")
        || t.starts_with("note: run with `RUST_BACKTRACE")
        || t.starts_with("test session starts") || t.starts_with("Traceback (most recent call last)")
        || t.starts_with("platform ") || t.starts_with("rootdir:") || t.starts_with("collecting ")
        || !t.is_empty() && t.chars().all(|c| matches!(c, '^' | '~' | ' '))
}

pub(super) fn without_noise(lines: &[String]) -> Vec<String> {
    let kept: Vec<String> = lines.iter().filter(|l| !is_noise(l)).cloned().collect();
    if kept.is_empty() { lines.to_vec() } else { kept }
}

/// A passing test line whose name merely contains a word like "error" (`test x_keeps_errors ... ok`).
pub(super) fn is_pass_line(l: &str) -> bool {
    let t = l.trim_end();
    t.ends_with(" ... ok") || t.ends_with(" PASSED") || t.contains(" PASSED ")
}

/// When something failed, passing-test lines only add noise: drop them and say how many.
pub(super) fn without_passes(lines: &[String]) -> Vec<String> {
    let kept: Vec<String> = lines.iter().filter(|l| !is_pass_line(l)).cloned().collect();
    let dropped = lines.len() - kept.len();
    if dropped < 3 { return lines.to_vec(); }
    let mut out = kept;
    let at = out.iter().position(|l| score(l) >= 2).unwrap_or(0).min(out.len());
    out.insert(at, format!("({dropped} passing-test lines omitted)"));
    out
}

/// `cargo test` prints one `test result:` line per suite; add them up into one line.
pub(super) fn sum_test_results(lines: &[String]) -> Option<String> {
    let (mut p, mut f, mut ig, mut suites) = (0u64, 0u64, 0u64, 0u64);
    for l in lines.iter().filter(|l| l.starts_with("test result:")) {
        suites += 1;
        for part in l.split(';') {
            let words: Vec<&str> = part.split_whitespace().collect();
            let Some(k) = words.iter().position(|w| matches!(*w, "passed" | "failed" | "ignored")) else { continue };
            let Some(n) = k.checked_sub(1).and_then(|k| words[k].parse::<u64>().ok()) else { continue };
            match words[k] { "passed" => p += n, "failed" => f += n, _ => ig += n }
        }
    }
    (suites > 0).then(|| {
        let mut parts = vec![format!("{p} passed")];
        if f > 0 { parts.push(format!("{f} failed")); }
        if ig > 0 { parts.push(format!("{ig} ignored")); }
        format!("test result: {}. {} ({suites} suites)", if f > 0 { "FAILED" } else { "ok" }, parts.join(", "))
    })
}

/// Successful run: the aggregated counts, other summary lines and the last lines, nothing else.
pub(super) fn summarize_clean(lines: &[String], budget: usize) -> String {
    let agg = sum_test_results(lines);
    let lines: Vec<String> = lines.iter().filter(|l| !is_pass_line(l)).cloned().collect();
    let rest: Vec<String> = if agg.is_some() { lines.iter().filter(|l| !l.starts_with("test result:") && !l.starts_with("test ") && !l.starts_with("running ")).cloned().collect() } else { lines };
    let gs = collapse(&rest);
    let n = gs.len();
    let has_sum = gs.iter().any(|g| score(&g.text) == 2);
    let mut sel: BTreeSet<usize> = if has_sum { BTreeSet::new() } else { (n.saturating_sub(2)..n).collect() };
    for i in (0..n).rev() {
        if score(&gs[i].text) != 2 { continue; }
        let mut trial = sel.clone();
        trial.insert(i);
        if ntok(&render(&gs, &trial)) > budget { break; }
        sel = trial;
    }
    let body: String = sel.iter().map(|&i| {
        let g = &gs[i];
        let c = if g.count > 1 { format!("  [×{}]", g.count) } else { String::new() };
        format!("{}{c}\n", cap(g.text.trim_matches(|ch: char| ch == '=' || ch == '-' || ch == ' ')))
    }).collect();
    match agg { Some(a) => format!("{a}\n{body}"), None => body }
}
