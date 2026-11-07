//! mypy, ruff (check / format) and pytest: diagnostics with locations, counts per code, summary lines.
use super::util::*;

pub fn mypy(lines: &[String]) -> Option<String> {
    let err = re(r"^(.+?:\d+(?::\d+)?): (error|note): (.*?)(?:\s+\[([\w-]+)\])?$");
    let (mut out, mut codes) = (vec![], vec![]);
    for l in lines {
        if let Some(c) = err.captures(l) {
            if &c[2] == "error" {
                let code = c.get(4).map_or(String::new(), |m| format!(" [{}]", m.as_str()));
                if let Some(m) = c.get(4) { codes.push(m.as_str().to_string()); }
                out.push(format!("{}: {}{}", &c[1], short(&c[3], 200), code));
            } else if out.last().map_or(false, |p: &String| !p.contains(" note: ")) && out.iter().rev().take(2).filter(|p| p.contains(" note: ")).count() < 2 {
                out.push(format!("  note: {}", short(&c[3], 120)));
            }
        }
    }
    let tail = grep(lines, r"^(Found \d+ errors?|Success: no issues)");
    if out.is_empty() && tail.is_empty() { return None; }
    let mut v = cap(out, 60);
    if codes.len() > 5 { v.push(format!("by code: {}", tally(&codes))); }
    v.extend(tail);
    finish(v)
}

pub fn ruff_check(lines: &[String]) -> Option<String> {
    let concise = re(r"^(\S+?:\d+:\d+): ([A-Z]+\d+) (.*)$");
    let full_head = re(r"^([A-Z]{1,5}\d{2,4}) (?:\[\*\] )?(.*)$");
    let arrow = re(r"^\s*--> (\S+)$");
    let (mut out, mut codes) = (vec![], vec![]);
    let mut pending: Option<(String, String)> = None;
    for l in lines {
        if let Some(c) = concise.captures(l) {
            codes.push(c[2].to_string());
            out.push(format!("{} {} {}", &c[1], &c[2], short(&c[3], 160)));
        } else if let Some(c) = full_head.captures(l) {
            pending = Some((c[1].to_string(), short(&c[2], 160)));
        } else if let (Some(c), Some((code, msg))) = (arrow.captures(l), pending.take()) {
            codes.push(code.clone());
            out.push(format!("{} {} {}", &c[1], code, msg));
        }
    }
    let tail = grep(lines, r"^(Found \d+ errors?|All checks passed|\[\*\] \d+ fixable|No fixes available|\d+ fixable)");
    if out.is_empty() && tail.is_empty() { return None; }
    let mut v = cap(out, 60);
    if codes.len() > 5 { v.push(format!("by rule: {}", tally(&codes))); }
    v.extend(tail);
    finish(v)
}

pub fn ruff_format(lines: &[String]) -> Option<String> {
    let files = grep(lines, r"^Would reformat: ");
    let tail = grep(lines, r"files? (reformatted|left unchanged|would be reformatted|already formatted)|^\d+ files?");
    if files.is_empty() && tail.is_empty() { return None; }
    let mut v = cap(files, 30);
    v.extend(tail);
    finish(v)
}

pub fn pytest(lines: &[String]) -> Option<String> {
    let last = re(r"^=+ .*\b(passed|failed|error|errors|skipped|no tests ran|deselected|warnings?)\b.* in [\d.]+s.*=+$");
    let sum = lines.iter().rev().find(|l| last.is_match(l))?;
    let start = |l: &str| re(r"^_{3,} .+ _{3,}$").is_match(l);
    let stop = |l: &str| re(r"^={3,} ").is_match(l);
    let drop = |l: &str| !(l.starts_with("E ") || l.starts_with("E\t") || re(r"^\S+\.py:\d+: ").is_match(l));
    let mut out = blocks(lines, &start, &stop, 10, &drop);
    out.extend(grep(lines, r"^(FAILED|ERROR) "));
    out.push(sum.trim_matches(|c| c == '=' || c == ' ').to_string());
    finish(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::util::t::*;

    #[test]
    fn mypy_errors_and_tally() {
        let mut s = String::new();
        for i in 0..7 { s += &format!("src/m{i}.py:{}: error: Incompatible types in assignment (expression has type \"str\", variable has type \"int\")  [assignment]\n", i + 1); }
        s += "src/m0.py:1: note: See https://mypy.rtfd.io/\nFound 7 errors in 7 files (checked 40 source files)\n";
        let l = lines(&s);
        let o = small(mypy(&l), &l);
        assert!(o.contains("src/m3.py:4: Incompatible types") && o.contains("by code: assignment x7") && o.contains("Found 7 errors"));
        assert!(mypy(&lines("Success: no issues found in 3 source files")).unwrap().contains("Success"));
    }

    #[test]
    fn ruff_check_both_formats() {
        let concise = lines("src/a.py:1:8: F401 [*] `os` imported but unused\nsrc/a.py:9:1: E501 Line too long (120 > 88)\nFound 2 errors.\n[*] 1 fixable with the `--fix` option.");
        let o = ruff_check(&concise).unwrap();
        assert!(o.contains("src/a.py:1:8 F401") && o.contains("Found 2 errors.") && o.contains("1 fixable"));
        let full = lines("F401 [*] `os` imported but unused\n --> src/a.py:1:8\n  |\n1 | import os\n  |        ^^\n  |\nhelp: Remove unused import\n\nFound 1 error.\n");
        let o = ruff_check(&full).unwrap();
        assert!(o.contains("src/a.py:1:8 F401") && !o.contains("import os") && !o.contains("help:"));
    }

    #[test]
    fn ruff_format_summary() {
        let l = lines("Would reformat: src/a.py\nWould reformat: src/b.py\n2 files would be reformatted, 30 files already formatted\n");
        let o = ruff_format(&l).unwrap();
        assert!(o.contains("Would reformat: src/b.py") && o.contains("2 files would be reformatted"));
    }

    #[test]
    fn pytest_failure_summary() {
        let mut s = String::from("============================= test session starts =============================\nplatform linux -- Python 3.11\ncollected 30 items\n\ntests/test_a.py ........................ [ 80%]\ntests/test_b.py ..F... [100%]\n\n=================================== FAILURES ===================================\n_________________________________ test_total ___________________________________\n\n    def test_total():\n        cart = Cart()\n>       assert cart.total() == 5\nE       assert 4 == 5\nE        +  where 4 = total()\n\ntests/test_b.py:12: AssertionError\n=========================== short test summary info ============================\nFAILED tests/test_b.py::test_total - assert 4 == 5\n");
        s += "========================= 1 failed, 29 passed in 0.31s =========================\n";
        let l = lines(&s);
        let o = small(pytest(&l), &l);
        assert!(o.contains("test_total") && o.contains("E       assert 4 == 5") && o.contains("tests/test_b.py:12: AssertionError"));
        assert!(o.contains("FAILED tests/test_b.py::test_total") && o.contains("1 failed, 29 passed in 0.31s"));
        assert!(!o.contains("platform linux") && !o.contains("cart = Cart()"));
    }
}
