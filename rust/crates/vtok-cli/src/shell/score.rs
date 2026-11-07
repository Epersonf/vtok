//! How much a line matters: 3 = error, 2 = summary, 1 = warning, 0 = chatter.
const ERR: &[&str] = &["error", "fail", "panic", "exception", "traceback", "fatal", "denied", "cannot ", "can't ", "not found", "assertion", "assert ", "✗", "segmentation", "abort", "timed out", "timeout", "unresolved"];
const SUM: &[&str] = &["test result", " passed", "passed;", "tests ran", "ran ", "summary", "total", "finished", "complete", "success", "test suites:", "failures:", "errors:", "real\t", "elapsed"];
const WARN: &[&str] = &["warning", "warn:", "deprecated"];

/// Drop "zero" counters so `0 failed` / `errors: 0` do not look like failures (`10 failed` is kept).
fn without_zero_counts(l: &str) -> String {
    let mut s = l.to_string();
    for w in ["failed", "failures", "failure", "errors", "error", "warnings", "warning", "errored"] {
        for pat in [format!("0 {w}"), format!("{w}: 0"), format!("{w}=0"), format!("{w} = 0")] {
            let mut from = 0;
            while let Some(i) = s[from..].find(&pat) {
                let at = from + i;
                let before_digit = s[..at].chars().last().map_or(false, |c| c.is_ascii_digit() || c == '.');
                let after_digit = s[at + pat.len()..].chars().next().map_or(false, |c| c.is_ascii_digit());
                if before_digit || after_digit { from = at + pat.len(); continue; }
                s.replace_range(at..at + pat.len(), "");
                from = at;
            }
        }
    }
    s
}

pub(super) fn score(line: &str) -> u8 {
    let l = without_zero_counts(&line.to_lowercase());
    if ERR.iter().any(|k| l.contains(k)) { 3 }
    else if SUM.iter().any(|k| l.contains(k)) || l.trim_start().starts_with("tests:") { 2 }
    else if WARN.iter().any(|k| l.contains(k)) { 1 }
    else { 0 }
}
