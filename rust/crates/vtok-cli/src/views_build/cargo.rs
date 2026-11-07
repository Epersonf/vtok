//! cargo build/check/clippy/doc/test/nextest/bench: a green run is one line (`cargo_green`); otherwise diagnostics as
//! `level: msg @ file:line` (warnings grouped), failing tests with their panic output, one aggregated test line, the
//! `Finished` line.
use super::cargo_parse::Parsed;
use super::{finish, uniq};

pub fn view(sub: &str, lines: &[String]) -> Option<String> {
    if !matches!(sub, "build" | "b" | "check" | "c" | "clippy" | "test" | "t" | "nextest" | "bench" | "doc" | "run" | "r" | "install" | "fix") {
        return None;
    }
    let mut p = Parsed::parse(lines);
    if !p.known { return None; }
    if let Some(one) = super::cargo_green::line(sub, &p) { return finish_one(one, lines); }
    uniq(&mut p.errors);
    let mut out = std::mem::take(&mut p.errors);
    p.warns.render(&mut out);
    for b in &p.blocks {
        out.push(b[0].clone());
        out.extend(b[1..].iter().take(11).cloned());
        if b.len() > 12 { out.push("  ...".into()); }
    }
    for n in p.names.iter().filter(|n| !p.blocks.iter().any(|b| b[0].contains(n.as_str()))) { out.push(format!("FAILED {n}")); }
    out.extend(std::mem::take(&mut p.bench));
    if p.suites > 0 { out.push(format!("tests: {} passed, {} failed, {} ignored ({} suites)", p.passed, p.failed, p.ignored, p.suites)); }
    out.extend(p.nx_sum.take());
    out.extend(std::mem::take(&mut p.tail));
    if let Some(fin) = p.finished.take() { out.push(if p.compiled > 0 { format!("{fin} ({} crates)", p.compiled) } else { fin }); }
    finish(out, lines)
}

/// The one-line summary, when it is smaller than the output it summarises.
fn finish_one(one: String, input: &[String]) -> Option<String> {
    let before: usize = input.iter().map(|l| l.len() + 1).sum();
    (one.len() < before).then_some(one)
}
