//! vitest and jest: failing tests with assertion + location, summary counts; passing files dropped.
use super::util::*;

fn node_modules(l: &str) -> bool { l.contains("node_modules") }

pub fn vitest(lines: &[String]) -> Option<String> {
    if !has(lines, r"^\s*Test Files\s") { return None; }
    let start = |l: &str| l.trim_start().starts_with("FAIL ");
    let stop = |l: &str| l.starts_with('⎯') || re(r"^\s*(Test Files|Tests)\s").is_match(l);
    let drop = |l: &str| node_modules(l) || re(r"^\s*\d+\|").is_match(l);
    let mut out = blocks(lines, &start, &stop, 12, &drop);
    out.extend(grep(lines, r"^\s*(Test Files|Tests|Snapshots|Errors|Type Errors|Duration)\s"));
    finish(out)
}

pub fn jest(lines: &[String]) -> Option<String> {
    if !has(lines, r"^Test Suites:") { return None; }
    let mut out = collapse(grep(lines, r"^FAIL "));
    let start = |l: &str| l.trim_start().starts_with("● ") && !l.contains("Console") && !l.contains("Test suite failed");
    let stop = |l: &str| re(r"^(PASS|FAIL|Test Suites:|Summary of all)").is_match(l);
    // drop stack frames without a file:line, node_modules frames and the source code frame
    let drop = |l: &str| {
        let t = l.trim_start();
        (t.starts_with("at ") && !re(r":\d+:\d+\)?$").is_match(t)) || node_modules(l) || re(r"^\s*>?\s*\d*\s*\|").is_match(l)
    };
    out.extend(blocks(lines, &start, &stop, 12, &drop));
    out.extend(grep(lines, r"^(Test Suites|Tests|Snapshots|Time):"));
    finish(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::util::t::*;

    #[test]
    fn vitest_keeps_failures_and_summary() {
        let mut s = String::new();
        for i in 0..8 { s += &format!(" ✓ src/ok{i}.test.ts (12 tests) {i}ms\n"); }
        s += " ❯ src/b.test.ts (2 tests | 1 failed) 8ms\n   × b > fails 3ms\n\n⎯⎯⎯⎯⎯⎯ Failed Tests 1 ⎯⎯⎯⎯⎯⎯\n\n FAIL  src/b.test.ts > b > fails\nAssertionError: expected 1 to be 2 // Object.is equality\n\n- Expected\n+ Received\n\n- 2\n+ 1\n\n ❯ src/b.test.ts:10:5\n      8| it('fails', () => {\n      9|   const a = 1\n     10|   expect(a).toBe(2)\n ❯ node_modules/x/y.js:1:1\n\n⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯\n\n Test Files  1 failed | 8 passed (9)\n      Tests  1 failed | 90 passed (91)\n   Duration  1.2s\n";
        let l = lines(&s);
        let o = small(vitest(&l), &l);
        assert!(o.contains("FAIL  src/b.test.ts > b > fails") && o.contains("expected 1 to be 2") && o.contains("src/b.test.ts:10:5"));
        assert!(o.contains("Test Files  1 failed") && o.contains("Tests  1 failed"));
        assert!(!o.contains("ok3.test.ts") && !o.contains("node_modules"));
        assert!(vitest(&lines("hello")).is_none());
    }

    #[test]
    fn jest_keeps_failure_blocks_once() {
        let mut s = String::new();
        for i in 0..6 { s += &format!("PASS src/ok{i}.test.js\n"); }
        let block = "  ● math › adds\n\n    expect(received).toBe(expected) // Object.is equality\n\n    Expected: 3\n    Received: 2\n\n      10 |   it('adds', () => {\n    > 11 |     expect(add(1,1)).toBe(3);\n         |                      ^\n\n      at Object.<anonymous> (src/math.test.js:11:22)\n      at node_modules/jest-circus/build/run.js:1:1\n\n";
        s += "FAIL src/math.test.js\n"; s += block;
        s += "Summary of all failing tests\nFAIL src/math.test.js\n"; s += block;
        s += "Test Suites: 1 failed, 6 passed, 7 total\nTests:       1 failed, 20 passed, 21 total\nSnapshots:   0 total\nTime:        1.5 s\n";
        let l = lines(&s);
        let o = small(jest(&l), &l);
        assert_eq!(o.matches("● math › adds").count(), 1);
        assert!(o.contains("Expected: 3") && o.contains("Received: 2") && o.contains("src/math.test.js:11:22"));
        assert!(o.contains("Tests:       1 failed") && o.contains("FAIL src/math.test.js"));
        assert!(!o.contains("PASS src/ok2") && !o.contains("jest-circus") && !o.contains("expect(add"));
    }
}
