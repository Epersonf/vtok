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
        if let Some(one) = one_line(b) { out.push(one); continue; }
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

/// A libtest failure as one line, `name file:line message`, when the test printed nothing before its panic and the
/// message is short (an assertion's `left`/`right` included); anything else keeps its block.
fn one_line(b: &[String]) -> Option<String> {
    let name = b[0].strip_prefix("---- ")?.strip_suffix(" stdout ----")?;
    let at = b.iter().position(|l| l.contains(" panicked at "))?;
    let loc = b[at].split(" panicked at ").nth(1)?.trim().trim_end_matches(':');
    // `file:line:col` -> `file:line`
    let loc = match loc.rsplit_once(':') { Some((rest, col)) if rest.contains(':') && col.bytes().all(|c| c.is_ascii_digit()) => rest, _ => loc };
    let msg: Vec<&str> = b[at + 1..].iter().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
    if at > 1 || msg.is_empty() || msg.len() > 4 { return None; }
    Some(format!("{name} {loc}: {}", msg.join(" ")))
}

/// The one-line summary, when it is smaller than the output it summarises.
fn finish_one(one: String, input: &[String]) -> Option<String> {
    let before: usize = input.iter().map(|l| l.len() + 1).sum();
    (one.len() < before).then_some(one)
}
