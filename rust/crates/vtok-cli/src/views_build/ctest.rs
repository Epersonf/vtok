//! ctest: failed/not-run tests with their captured output, the failure list, pass-rate and total time.
//! Drops "Start N:" lines and every passing test line.
use super::{finish, trunc};
use regex::Regex;

pub fn view(lines: &[String]) -> Option<String> {
    let res = Regex::new(r"^\s*\d+/\d+ Test\s+#\d+: (\S+) [. ]*(\*\*\*[A-Za-z ]+?|Passed)(?:\s+[\d.]+ sec)?\s*$").unwrap();
    let (mut out, mut passed, mut known, mut budget, mut listing) = (Vec::<String>::new(), 0, false, 0, false);
    for l in lines {
        let t = l.trim_end();
        if let Some(c) = res.captures(t) {
            known = true;
            listing = false;
            if &c[2] == "Passed" { passed += 1; budget = 0; }
            else { out.push(format!("FAIL {} ({})", &c[1], c[2].trim_start_matches('*'))); budget = 15; }
        } else if t.starts_with("Test project") || t.trim_start().starts_with("Start ") || t.contains("Test command:") || t.contains("Test timeout") {
            known |= t.starts_with("Test project");
            budget = 0;
        } else if t.contains("tests passed,") || t.starts_with("Total Test time") || t.starts_with("Errors while running") || t.contains("No tests were found") {
            known = true;
            budget = 0;
            listing = false;
            out.push(t.to_string());
        } else if t.starts_with("The following tests") {
            known = true;
            budget = 0;
            listing = true;
            out.push(t.to_string());
        } else if listing && !t.trim().is_empty() {
            out.push(t.trim().to_string());
        } else if budget > 0 && !t.trim().is_empty() {
            budget -= 1;
            out.push(format!("  {}", trunc(t.trim(), 250)));
        }
    }
    if !known { return None; }
    if passed > 0 { out.insert(0, format!("({passed} passed)")); }
    finish(out, lines)
}
