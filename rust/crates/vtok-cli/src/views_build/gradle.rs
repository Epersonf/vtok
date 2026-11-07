//! gradle / gradlew: BUILD result, failed tasks, "what went wrong", compile errors (javac and kotlinc),
//! failing tests with message and first user frame. Drops task progress, warnings (counted) and hints.
use super::{finish, trunc, uniq};
use regex::Regex;

pub fn view(lines: &[String]) -> Option<String> {
    let javac = Regex::new(r"^(\S+\.(?:java|kt|scala|groovy)):(\d+):(?::\d+:)? (error|warning): (.*)$").unwrap();
    let tfail = Regex::new(r"^(\S.* > .*) FAILED$").unwrap();
    let lib = ["org.junit", "org.opentest4j", "java.base", "jdk.internal", "sun.reflect", "org.gradle", "kotlin.test", "junit.framework"];
    let (mut out, mut warns, mut tasks, mut known) = (Vec::<String>::new(), 0, 0, false);
    let mut i = 0;
    while i < lines.len() {
        let l = lines[i].trim_end();
        i += 1;
        if l.starts_with("> Task ") {
            known = true;
            if l.ends_with(" FAILED") { out.push(l.to_string()); } else { tasks += 1; }
        } else if l.starts_with("BUILD ") || l.starts_with("FAILURE:") || l.contains(" tests completed,") || l.contains("actionable task") {
            known = true;
            out.push(l.to_string());
        } else if l.starts_with("* What went wrong:") {
            while i < lines.len() && !lines[i].starts_with("* ") {
                if !lines[i].trim().is_empty() { out.push(format!("  {}", trunc(lines[i].trim(), 250))); }
                i += 1;
            }
        } else if let Some(c) = javac.captures(l) {
            known = true;
            if &c[3] == "error" {
                out.push(format!("{}:{}: error: {}", &c[1], &c[2], &c[4]));
                if lines.get(i + 1).is_some_and(|x| x.trim() == "^") { i += 2; }
            } else { warns += 1; }
        } else if l.starts_with("e: ") {
            known = true;
            out.push(trunc(l, 300));
        } else if l.starts_with("w: ") {
            warns += 1;
        } else if let Some(c) = tfail.captures(l) {
            known = true;
            out.push(format!("FAILED {}", &c[1]));
            let (mut msg, mut frame) = (0, false);
            while i < lines.len() && lines[i].starts_with("    ") {
                let t = lines[i].trim();
                if t.starts_with("at ") { if !frame && !lib.iter().any(|p| t.contains(p)) { out.push(format!("    {t}")); frame = true; } }
                else if msg < 6 { out.push(format!("    {}", trunc(t, 250))); msg += 1; }
                i += 1;
            }
        } else if l.contains("Configure project") || l.starts_with("Starting a Gradle") { known = true; }
    }
    if !known { return None; }
    uniq(&mut out);
    if warns > 0 { out.insert(0, format!("({warns} warnings dropped)")); }
    if tasks > 0 { out.insert(0, format!("({tasks} tasks ran)")); }
    finish(out, lines)
}
