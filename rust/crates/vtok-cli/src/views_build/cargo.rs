//! cargo build/check/clippy/doc/test/nextest/bench: diagnostics as `level: msg @ file:line` (warnings grouped),
//! failing tests with their panic output, one aggregated test line, the `Finished` line.
use super::{finish, uniq, Groups};
use regex::Regex;

pub fn view(sub: &str, lines: &[String]) -> Option<String> {
    if !matches!(sub, "build" | "b" | "check" | "c" | "clippy" | "test" | "t" | "nextest" | "bench" | "doc" | "run" | "r" | "install" | "fix") {
        return None;
    }
    let diag = Regex::new(r"^(warning|error)(\[\w+\])?: (.+)$").unwrap();
    let res = Regex::new(r"^test result: \w+\. (\d+) passed; (\d+) failed; (\d+) ignored").unwrap();
    let nx = Regex::new(r"^\s*(?:TRY \d+ )?(PASS|FAIL|SLOW|SKIP|LEAK|TIMEOUT|SIGABRT|SIGSEGV|START|RETRY|CANCEL)\s+\[").unwrap();
    let (mut errors, mut warns, mut tail) = (Vec::<String>::new(), Groups::default(), Vec::<String>::new());
    let (mut blocks, mut names, mut bench) = (Vec::<Vec<String>>::new(), Vec::<String>::new(), Vec::<String>::new());
    let (mut p, mut f, mut ig, mut suites, mut compiled, mut known) = (0usize, 0usize, 0usize, 0usize, 0usize, false);
    let (mut finished, mut nx_sum) = (None::<String>, None::<String>);
    let mut i = 0;
    while i < lines.len() {
        let l = lines[i].as_str();
        let t = l.trim_start();
        i += 1;
        if let Some(c) = diag.captures(l) {
            known = true;
            let msg = &c[3];
            if msg.starts_with("could not compile") || msg.starts_with("aborting") || msg.starts_with("test failed") {
                tail.push(l.to_string());
                continue;
            }
            if (msg.starts_with('`') && msg.contains(" generated ")) || msg.starts_with("unused manifest") || msg.starts_with("build failed, waiting") {
                continue;
            }
            let loc = lines[i..].iter().take(6).find_map(|x| x.trim_start().strip_prefix("--> ")).unwrap_or("");
            let code = c.get(2).map_or("", |m| m.as_str());
            if &c[1] == "error" {
                errors.push(if loc.is_empty() { format!("error{code}: {msg}") } else { format!("error{code}: {msg} @ {loc}") });
            } else {
                warns.add(&format!("warning{code}: {msg}"), loc);
            }
        } else if let Some(c) = res.captures(t) {
            known = true;
            suites += 1;
            let n = |k: usize| c[k].parse::<usize>().unwrap_or(0);
            (p, f, ig) = (p + n(1), f + n(2), ig + n(3));
        } else if t.starts_with("Compiling ") || t.starts_with("Checking ") || t.starts_with("Documenting ") {
            known = true;
            compiled += 1;
        } else if t.starts_with("Finished ") {
            known = true;
            finished = Some(t.to_string());
        } else if t.starts_with("Running ") || t.starts_with("Doc-tests") || t.starts_with("running ") || t.starts_with("Starting ") {
            known = true;
        } else if t.starts_with("Summary [") {
            known = true;
            nx_sum = Some(t.to_string());
        } else if let Some(c) = nx.captures(l) {
            known = true;
            if matches!(&c[1], "FAIL" | "TIMEOUT" | "SIGABRT" | "SIGSEGV" | "LEAK") {
                let mut b = vec![l.trim().to_string()];
                while i < lines.len() && !nx.is_match(&lines[i]) && !lines[i].starts_with("-----") && !lines[i].trim_start().starts_with("Summary") && b.len() < 14 {
                    if !lines[i].trim().is_empty() { b.push(lines[i].trim_end().to_string()); }
                    i += 1;
                }
                if !blocks.iter().any(|x| x[0] == b[0]) { blocks.push(b); }
            }
        } else if t.starts_with("test ") && t.ends_with("... FAILED") {
            known = true;
            names.push(t[5..t.len() - 11].to_string());
        } else if t.starts_with("---- ") && t.ends_with(" ----") {
            known = true;
            let mut b = vec![t.to_string()];
            while i < lines.len() {
                let n = lines[i].trim_end();
                if n.starts_with("---- ") || n == "failures:" || n.starts_with("test result:") { break; }
                if !n.is_empty() && !n.starts_with("note: run with `RUST_BACKTRACE") { b.push(format!("  {}", n.trim_start())); }
                i += 1;
            }
            blocks.push(b);
        } else if t.contains(" bench: ") {
            bench.push(t.to_string());
        } else if t.contains("time:") && t.contains('[') {
            let name = if t.starts_with("time:") && i >= 2 { lines[i - 2].trim() } else { "" };
            bench.push(format!("{name} {t}").trim().to_string());
        }
    }
    if !known { return None; }
    uniq(&mut errors);
    let mut out = errors;
    warns.render(&mut out);
    for b in &blocks {
        out.push(b[0].clone());
        out.extend(b[1..].iter().take(11).cloned());
        if b.len() > 12 { out.push("  ...".into()); }
    }
    for n in names.iter().filter(|n| !blocks.iter().any(|b| b[0].contains(n.as_str()))) { out.push(format!("FAILED {n}")); }
    out.extend(bench);
    if suites > 0 { out.push(format!("tests: {p} passed, {f} failed, {ig} ignored ({suites} suites)")); }
    out.extend(nx_sum);
    out.extend(tail);
    if let Some(fin) = finished { out.push(if compiled > 0 { format!("{fin} ({compiled} crates)") } else { fin }); }
    finish(out, lines)
}
