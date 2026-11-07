//! vitest and jest: failing tests with assertion + location, summary counts; passing files dropped. The vitest view also
//! drops what only repeats something printed next to it: the code-frame caret, an expected/received pair whose values the
//! assertion message already states, chai's `// Object.is equality` note, the `AssertionError: ` prefix of its
//! `expected ...` messages, and the timing breakdown after `Duration`.
use super::util::*;

fn node_modules(l: &str) -> bool { l.contains("node_modules") }

/// A value as a diff prints it (`"a b"`) or the message does (`'a b'`), without the quotes.
fn bare(v: &str) -> &str { v.trim().trim_matches(|c| c == '"' || c == '\'') }

/// `v` occurs in `msg` as a whole token: `0` is in `expected 5 to be +0`, `3` is not in `expected 12 to be 30`.
fn states(msg: &str, v: &str) -> bool {
    let edge = |c: Option<char>| c.map_or(true, |c| !c.is_alphanumeric());
    !v.is_empty() && msg.match_indices(v).any(|(i, _)| edge(msg[..i].chars().next_back()) && edge(msg[i + v.len()..].chars().next()))
}

/// Line range `[from, to)` of an expected/received pair that only repeats values the message above it states, in the two
/// scalar shapes vitest prints: `Expected: "a"` / `Received: "b"`, and `- Expected` / `+ Received` over `- a` / `+ b`.
/// Any bigger diff (context lines, objects, snapshots) is not a pair and stays.
fn repeated_pair(b: &[String]) -> Option<(usize, usize)> {
    let i = b.iter().position(|l| l.starts_with("Expected: ") || l.starts_with("- Expected"))?;
    let to = b[i..].iter().position(|l| l.trim_start().starts_with('❯') || l.trim() == "...").map_or(b.len(), |p| i + p);
    let (x, y) = match &b[i..to] {
        [e, r] if e.starts_with("Expected: ") && r.starts_with("Received: ") => (&e[10..], &r[10..]),
        [e, r, x, y] if e.starts_with("- Expected") && r.starts_with("+ Received") && x.starts_with("- ") && y.starts_with("+ ") => (&x[2..], &y[2..]),
        _ => return None,
    };
    let msg = b.get(1..i)?.join(" ");
    (states(&msg, bare(x)) && states(&msg, bare(y))).then_some((i, to))
}

/// One failure block: header and location without their indentation, the message without its filler, and the
/// expected/received pair gone when the message already says it. Diff lines keep their spacing.
fn tidy(mut b: Vec<String>) -> Vec<String> {
    for l in b.iter_mut() {
        let new = {
            let t = l.trim_start();
            if let Some(r) = t.strip_prefix("FAIL ") { format!("FAIL {}", r.trim_start()) }
            else if t.starts_with('❯') { t.to_string() }
            else { l.strip_prefix("AssertionError: ").filter(|r| r.starts_with("expected ")).unwrap_or(l.as_str()).trim_end_matches(" // Object.is equality").to_string() }
        };
        *l = new;
    }
    if let Some((from, to)) = repeated_pair(&b) { b.drain(from..to); }
    b
}

pub fn vitest(lines: &[String]) -> Option<String> {
    if !has(lines, r"^\s*Test Files\s") { return None; }
    let (summary, frame) = (re(r"^\s*(Test Files|Tests)\s"), re(r"^\s*(\d+\||\|\s*\^)"));
    let start = |l: &str| l.trim_start().starts_with("FAIL ");
    let stop = |l: &str| l.starts_with('⎯') || summary.is_match(l);
    let drop = |l: &str| node_modules(l) || frame.is_match(l);
    let (mut out, mut cur) = (vec![], vec![]);
    for l in blocks(lines, &start, &stop, 12, &drop) {
        if start(&l) && !cur.is_empty() { out.extend(tidy(std::mem::take(&mut cur))); }
        cur.push(l);
    }
    out.extend(tidy(cur));
    for l in grep(lines, r"^\s*(Test Files|Tests|Snapshots|Errors|Type Errors|Duration)\s") {
        out.push(if l.starts_with("Duration") { l.split(" (").next().unwrap_or(&l).to_string() } else { l });
    }
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
