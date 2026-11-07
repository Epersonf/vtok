//! oxlint and biome: every error location kept, warnings collapsed per rule (eslint has its own module, `js_eslint`).
use super::util::*;
use crate::diag::cap_classes;

pub fn oxlint(lines: &[String]) -> Option<String> {
    let head = re(r"^\s*([x!×⚠]) (\S+?): (.*)$");
    let loc = re(r"^\s*[,╭]-\[(.+?)\]");
    let (mut out, mut warns) = (vec![], vec![]);
    let mut pending: Option<(bool, String, String)> = None;
    for l in lines {
        if let Some(c) = head.captures(l) { pending = Some((matches!(&c[1], "x" | "×"), c[2].to_string(), format!("{}: {}", &c[2], short(&c[3], 160)))); }
        else if let (Some(c), Some((err, rule, msg))) = (loc.captures(l), pending.as_ref()) {
            if *err { out.push((rule.clone(), format!("{} {}", &c[1], msg))); } else { warns.push(rule.clone()); }
            pending = None;
        }
    }
    let tail = grep(lines, r"^(Found \d+|Finished in)");
    if out.is_empty() && warns.is_empty() && tail.is_empty() { return None; }
    let mut v = cap_classes(out, 80);
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
            if matches!(&c[1], "×" | "✖") { out.push((rule.clone(), format!("{loc} {rule}: {}", short(&c[2], 160)))); } else { warns.push(rule.clone()); }
            pending = None;
        }
    }
    let tail = grep(lines, r"^(Checked \d+|Found \d+|Formatted \d+|Fixed \d+)");
    if out.is_empty() && warns.is_empty() && tail.is_empty() { return None; }
    let mut v = cap_classes(out, 80);
    if !warns.is_empty() { v.push(format!("{} warning(s): {}", warns.len(), tally(&warns))); }
    v.extend(tail);
    finish(v)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::util::t::*;

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
