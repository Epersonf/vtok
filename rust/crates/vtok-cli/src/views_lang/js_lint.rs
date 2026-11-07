//! eslint (stylish), oxlint and biome: every error location kept, warnings collapsed per rule.
use super::util::*;

pub fn eslint(lines: &[String]) -> Option<String> {
    let item = re(r"^\s+(\d+):(\d+)\s+(error|warning)\s+(.*?)\s{2,}(\S+)\s*$");
    let (mut file, mut errs, mut warns) = (String::new(), vec![], vec![]);
    let mut shown = String::new();
    for l in lines {
        if let Some(c) = item.captures(l) {
            if &c[3] == "error" {
                if shown != file { errs.push(file.clone()); shown = file.clone(); }
                errs.push(format!("  {}:{} {} [{}]", &c[1], &c[2], short(&c[4], 160), &c[5]));
            } else { warns.push(c[5].to_string()); }
        } else if !l.starts_with(' ') && !l.trim().is_empty() && !l.starts_with('✖') && !l.starts_with("0 errors") { file = l.trim().to_string(); }
    }
    let summary = grep(lines, r"^✖ \d+ problems?|^\d+ problems?");
    if errs.is_empty() && warns.is_empty() && summary.is_empty() { return None; }
    let mut out = cap(errs, 80);
    if !warns.is_empty() { out.push(format!("{} warning(s): {}", warns.len(), tally(&warns))); }
    out.extend(summary);
    out.extend(grep(lines, r"potentially fixable with the `--fix`"));
    finish(out)
}

pub fn oxlint(lines: &[String]) -> Option<String> {
    let head = re(r"^\s*([x!×⚠]) (\S+?): (.*)$");
    let loc = re(r"^\s*[,╭]-\[(.+?)\]");
    let (mut out, mut warns) = (vec![], vec![]);
    let mut pending: Option<(bool, String)> = None;
    for l in lines {
        if let Some(c) = head.captures(l) { pending = Some((matches!(&c[1], "x" | "×"), format!("{}: {}", &c[2], short(&c[3], 160)))); }
        else if let (Some(c), Some((err, msg))) = (loc.captures(l), pending.as_ref()) {
            if *err { out.push(format!("{} {}", &c[1], msg)); } else { warns.push(msg.split(':').next().unwrap_or("").to_string()); }
            pending = None;
        }
    }
    let tail = grep(lines, r"^(Found \d+|Finished in)");
    if out.is_empty() && warns.is_empty() && tail.is_empty() { return None; }
    let mut v = cap(out, 80);
    if !warns.is_empty() { v.push(format!("{} warning(s): {}", warns.len(), tally(&warns))); }
    v.extend(tail);
    finish(v)
}

pub fn biome(lines: &[String]) -> Option<String> {
    let head = re(r"^(\S+:\d+:\d+) (\S+)(\s+.*)?\s+[━─]{3,}");
    let msg = re(r"^\s+([×!ℹ✖]|i)\s+(.*)$");
    let (mut out, mut warns) = (vec![], vec![]);
    let mut pending: Option<(String, String)> = None;
    for l in lines {
        if let Some(c) = head.captures(l) { pending = Some((c[1].to_string(), c[2].to_string())); }
        else if let (Some(c), Some((loc, rule))) = (msg.captures(l), pending.as_ref()) {
            if matches!(&c[1], "×" | "✖") { out.push(format!("{loc} {rule}: {}", short(&c[2], 160))); } else { warns.push(rule.clone()); }
            pending = None;
        }
    }
    let tail = grep(lines, r"^(Checked \d+|Found \d+|Formatted \d+|Fixed \d+)");
    if out.is_empty() && warns.is_empty() && tail.is_empty() { return None; }
    let mut v = cap(out, 80);
    if !warns.is_empty() { v.push(format!("{} warning(s): {}", warns.len(), tally(&warns))); }
    v.extend(tail);
    finish(v)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::util::t::*;

    #[test]
    fn eslint_errors_kept_warnings_collapsed() {
        let mut s = String::from("/app/src/a.js\n  10:5  error  'x' is defined but never used  no-unused-vars\n");
        for i in 0..6 { s += &format!("  {}:1  warning  Unexpected console statement  no-console\n", 20 + i); }
        s += "\n/app/src/b.js\n  3:9  warning  Missing semicolon  semi\n\n✖ 8 problems (1 error, 7 warnings)\n  0 errors and 1 warning potentially fixable with the `--fix` option.\n";
        let l = lines(&s);
        let o = small(eslint(&l), &l);
        assert!(o.contains("/app/src/a.js") && o.contains("10:5 'x' is defined but never used [no-unused-vars]"));
        assert!(o.contains("7 warning(s): no-console x6, semi x1") && o.contains("✖ 8 problems"));
        assert!(!o.contains("20:1") && !o.contains("/app/src/b.js"));
    }

    #[test]
    fn oxlint_keeps_error_locations() {
        let s = "  x eslint(no-debugger): `debugger` statement is not allowed\n   ,-[src/a.js:4:3]\n 3 | foo();\n 4 | debugger;\n   : ^^^^^^^^^ \n 5 | bar();\n   `----\n  help: Delete this code.\n\n  ! eslint(no-unused-vars): Variable 'y' is declared but never used.\n   ,-[src/b.js:1:7]\n 1 | const y = 1;\n   `----\n  help: Consider removing.\n\nFound 1 warning and 1 error.\nFinished in 5ms on 2 files with 90 rules using 8 threads.\n";
        let l = lines(s);
        let o = small(oxlint(&l), &l);
        assert!(o.contains("src/a.js:4:3 eslint(no-debugger)") && o.contains("1 warning(s)") && o.contains("Found 1 warning and 1 error."));
        assert!(!o.contains("help:") && !o.contains("foo();"));
    }

    #[test]
    fn biome_keeps_error_locations() {
        let s = "src/a.ts:3:7 lint/correctness/noUnusedVariables  FIXABLE  ━━━━━━━━━━━━━━━━\n\n  ! This variable is unused.\n\n  > 3 │ const x = 1;\n      │       ^\n\nsrc/b.ts:9:1 lint/suspicious/noDoubleEquals ━━━━━━━━━━━━━━━━\n\n  × Use === instead of ==.\n\n  > 9 │ a == b\n\nChecked 12 files in 4ms. No fixes applied.\nFound 1 error.\nFound 1 warning.\n";
        let l = lines(s);
        let o = small(biome(&l), &l);
        assert!(o.contains("src/b.ts:9:1 lint/suspicious/noDoubleEquals: Use === instead of ==.") && o.contains("Found 1 error."));
        assert!(o.contains("1 warning(s): lint/correctness/noUnusedVariables") && !o.contains("const x"));
    }
}
