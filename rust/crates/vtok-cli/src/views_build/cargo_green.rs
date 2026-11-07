//! A cargo run that went fine, in one line: `cargo test: 166 passed (3 suites), 4 warnings`. The counts are the facts;
//! the warnings, timings and crate lists stay in the stored output (`vtok output --grep warning`).
use super::cargo_parse::Parsed;

/// `Summary [ 0.5s] 3 tests run: 3 passed, 0 skipped` -> `3 passed` (zero counters dropped).
fn nextest_counts(sum: &str) -> String {
    let after = sum.split_once("tests run:").or_else(|| sum.split_once("test run:")).map_or("", |x| x.1);
    after.split(',').map(str::trim).filter(|p| !p.is_empty() && !p.starts_with("0 ")).collect::<Vec<_>>().join(", ")
}

fn warning_count(n: usize) -> String { if n == 1 { "1 warning".into() } else { format!("{n} warnings") } }

/// None when the run is not one of the plain build/test kinds or something failed.
pub(super) fn line(sub: &str, p: &Parsed) -> Option<String> {
    if !p.green() { return None; }
    let name = match sub { "t" => "test", "b" => "build", "c" => "check", s => s };
    let mut parts: Vec<String> = match name {
        "test" if p.suites == 0 => return None,
        "test" => {
            let mut v = vec![format!("{} passed", p.passed)];
            if p.ignored > 0 { v.push(format!("{} ignored", p.ignored)); }
            v
        }
        "nextest" => vec![p.nx_sum.as_deref().map(nextest_counts).filter(|s| !s.is_empty())?],
        "build" | "check" | "doc" => vec!["ok".into()],
        "clippy" if p.warns.total() == 0 => vec!["ok".into()],
        _ => return None,
    };
    let mut out = format!("cargo {name}: {}", parts.remove(0));
    for x in parts { out.push_str(&format!(", {x}")); }
    if name == "test" && p.suites > 1 { out.push_str(&format!(" ({} suites)", p.suites)); }
    if p.warns.total() > 0 { out.push_str(&format!(", {}", warning_count(p.warns.total()))); }
    if p.other > 0 { out.push_str(&format!(", {} lines of other output omitted", p.other)); }
    Some(out + "\n")
}
