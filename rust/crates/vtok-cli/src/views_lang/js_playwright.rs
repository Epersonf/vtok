//! playwright test (list/line reporters): numbered failures with error + location, final counts.
use super::util::*;

pub fn playwright(lines: &[String]) -> Option<String> {
    let summary = re(r"^\s*\d+ (passed|failed|flaky|skipped|did not run)");
    if !summary.is_match(&lines.iter().rev().find(|l| summary.is_match(l)).cloned().unwrap_or_default()) { return None; }
    let numbered = re(r"^\s+\d+\) ");
    let start = |l: &str| numbered.is_match(l);
    let stop = |l: &str| summary.is_match(l) || l.trim() == "Slow test file:" ;
    // drop code frames, call logs and stack frames inside node_modules
    let drop = |l: &str| re(r"^\s+(>?\s*\d+ \||\|)").is_match(l) || l.contains("node_modules") || l.trim_start().starts_with("attachment #") || l.trim_start().starts_with("Usage:") || l.trim_start().starts_with("Error Context");
    let mut out = blocks(lines, &start, &stop, 14, &drop);
    // the closing list: "N failed" + "[project] > file" lines, then "M passed (5s)"
    let first = lines.iter().position(|l| re(r"^\s*\d+ (failed|flaky)").is_match(l));
    let tail: Vec<String> = match first {
        Some(i) => lines[i..].iter().filter(|l| !l.trim().is_empty()).map(|l| l.trim_end().to_string()).collect(),
        None => grep(lines, r"^\s*\d+ (passed|skipped|did not run)"),
    };
    out.extend(cap(tail, 25));
    finish(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::util::t::*;

    #[test]
    fn keeps_failure_and_counts() {
        let mut s = String::from("Running 12 tests using 4 workers\n");
        for i in 1..=10 { s += &format!("  ✓  {i} [chromium] › a.spec.ts:{i}:1 › works {i} (1.{i}s)\n"); }
        s += "  ✘  11 [chromium] › b.spec.ts:5:1 › fails (2.0s)\n\n\n  1) [chromium] › b.spec.ts:5:1 › fails ──────────\n\n    Error: expect(received).toBe(expected)\n\n    Expected: \"Home\"\n    Received: \"Login\"\n\n      5 |   await page.goto('/');\n    > 6 |   expect(await page.title()).toBe('Home');\n        |                              ^\n\n        at b.spec.ts:6:32\n\n  1 failed\n    [chromium] › b.spec.ts:5:1 › fails\n  11 passed (6s)\n";
        let l = lines(&s);
        let o = small(playwright(&l), &l);
        assert!(o.contains("1) [chromium] › b.spec.ts:5:1 › fails") && o.contains("Expected: \"Home\"") && o.contains("b.spec.ts:6:32"));
        assert!(o.contains("1 failed") && o.contains("11 passed (6s)"));
        assert!(!o.contains("works 3") && !o.contains("await page.goto"));
        assert!(playwright(&lines("nothing here")).is_none());
    }
}
