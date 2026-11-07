//! mvn / mvnw: errors, failing tests (with their assertion line), test totals, reactor failures, BUILD result.
//! Drops [INFO] chatter, downloads, passing test classes, stack frames and repeated warnings.
use super::{finish, trunc, uniq};
use regex::Regex;

pub fn view(lines: &[String]) -> Option<String> {
    let lvl = Regex::new(r"^\[(INFO|WARNING|ERROR|DEBUG)\] ?(.*)$").unwrap();
    let zero = Regex::new(r"Failures: 0, Errors: 0").unwrap();
    let noise = ["-> [Help", "[Help ", "To see the full stack trace", "Re-run Maven", "For more information about the errors", "Failed to execute goal", "After correcting"];
    let (mut out, mut warns, mut ok_classes, mut ok_mods, mut known) = (Vec::<String>::new(), 0, 0, 0, false);
    let mut i = 0;
    while i < lines.len() {
        let l = lines[i].trim_end();
        i += 1;
        let Some(c) = lvl.captures(l) else { continue };
        known = true;
        let (level, body) = (&c[1], c[2].trim_end());
        if body.contains("<<< FAILURE!") || body.contains("<<< ERROR!") {
            out.push(trunc(body, 200));
            out.extend(lines[i..].iter().take(2).filter(|x| !x.trim_start().starts_with("at ") && !x.starts_with('[') && !x.trim().is_empty()).map(|x| format!("  {}", trunc(x.trim(), 250))));
        } else if body.starts_with("Tests run:") {
            if !body.contains("Time elapsed") { out.push(body.to_string()); }
            else if zero.is_match(body) { ok_classes += 1; }
            else { out.push(body.to_string()); }
        } else if body.starts_with("BUILD ") || body.starts_with("Total time:") {
            out.push(body.to_string());
        } else if level == "INFO" && body.contains(" SUCCESS [") {
            ok_mods += 1;
        } else if level == "INFO" && (body.contains(" FAILURE [") || body.contains(" SKIPPED")) {
            out.push(body.to_string());
        } else if level == "WARNING" {
            warns += 1;
        } else if level == "ERROR" && !body.is_empty() {
            if body.starts_with("Failed to execute goal") { out.push(trunc(body, 300)); }
            else if !noise.iter().any(|n| body.contains(n)) { out.push(trunc(body, 300)); }
        }
    }
    if !known { return None; }
    uniq(&mut out);
    let mut head = Vec::new();
    if warns > 0 { head.push(format!("({warns} warnings dropped)")); }
    if ok_classes > 0 { head.push(format!("({ok_classes} passing test classes)")); }
    if ok_mods > 0 { head.push(format!("({ok_mods} modules SUCCESS)")); }
    head.extend(out);
    finish(head, lines)
}
