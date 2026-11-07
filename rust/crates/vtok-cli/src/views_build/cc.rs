//! make / ninja / cmake / gcc / g++ / clang: compiler errors with file:line, grouped warnings, linker errors,
//! make/ninja failure lines, CMake errors. Drops "[ 12%] Building", "Linking", directory banners, notes and code frames.
use super::{finish, trunc, uniq, Groups};
use regex::Regex;

pub fn view(lines: &[String]) -> Option<String> {
    let diag = Regex::new(r"^(.+?):(\d+)(?::(\d+))?: (fatal error|error|warning): (.*)$").unwrap();
    let mk = Regex::new(r"^(?:mingw32-)?(?:g?make)(?:\[\d+\])?: \*\*\*|^ninja: (?:build stopped|error)").unwrap();
    let (mut errors, mut warns, mut link, mut misc) = (Vec::<String>::new(), Groups::default(), Groups::default(), Vec::<String>::new());
    let (mut targets, mut known) = (0, false);
    let mut i = 0;
    while i < lines.len() {
        let l = lines[i].trim_end();
        i += 1;
        if let Some(c) = diag.captures(l) {
            known = true;
            let loc = if let Some(col) = c.get(3) { format!("{}:{}:{}", &c[1], &c[2], col.as_str()) } else { format!("{}:{}", &c[1], &c[2]) };
            if &c[4] == "warning" { warns.add(&format!("warning: {}", trunc(&c[5], 200)), &loc); }
            else { errors.push(format!("{loc}: error: {}", trunc(&c[5], 250))); }
        } else if let Some(p) = l.find("undefined reference to") {
            known = true;
            link.add(&format!("undefined reference to {}", &l[p + 22..].trim()), "");
        } else if l.contains("multiple definition of") || l.starts_with("collect2: error") || l.contains("ld returned") || l.contains("cannot find -l") {
            known = true;
            misc.push(trunc(l, 250));
        } else if mk.is_match(l) || l.starts_with("FAILED: ") || l.starts_with("-- Configuring incomplete") || l.contains("Build files have been written") {
            known = true;
            misc.push(trunc(l, 250));
        } else if l.starts_with("CMake Error") {
            known = true;
            misc.push(l.to_string());
            while i < lines.len() && !lines[i].trim().is_empty() && misc.len() < 40 { misc.push(format!("  {}", lines[i].trim())); i += 1; }
        } else if l.contains("Built target") {
            known = true;
            targets += 1;
        } else if l.contains("Building C") || l.contains("Linking ") || l.starts_with("make[") || l.starts_with("-- ") || l.contains("Scanning dependencies") {
            known = true;
        }
    }
    if !known { return None; }
    uniq(&mut errors);
    uniq(&mut misc);
    let (ne, nw) = (errors.len(), warns.total());
    let failed = ne > 0 || link.total() > 0 || misc.iter().any(|m| m.contains("***") || m.starts_with("FAILED") || m.starts_with("CMake Error"));
    let mut out = misc.iter().filter(|m| m.starts_with("CMake Error") || m.starts_with("  ")).cloned().collect::<Vec<_>>();
    out.extend(errors);
    link.render(&mut out);
    warns.render(&mut out);
    out.extend(misc.into_iter().filter(|m| !m.starts_with("CMake Error") && !m.starts_with("  ")));
    out.push(if failed { format!("build FAILED: {ne} compile errors, {nw} warnings") } else { format!("build ok ({nw} warnings, {targets} targets)") });
    finish(out, lines)
}
