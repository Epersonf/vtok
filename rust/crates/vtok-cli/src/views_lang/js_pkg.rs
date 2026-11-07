//! npm / pnpm / yarn / bun / deno: install, outdated, ls; test and run scripts are sniffed for the runner.
use super::util::*;
use super::{js_lint, js_playwright, js_test, ts_tsc};

pub fn pkg(tool: &str, args: &[String], lines: &[String]) -> Option<String> {
    let sub = args.iter().find(|a| !a.starts_with('-')).map(|s| s.as_str()).unwrap_or("");
    match sub {
        "install" | "i" | "add" | "ci" | "update" | "up" | "remove" | "rm" | "uninstall" => install(lines),
        "outdated" => outdated(lines),
        "ls" | "list" | "ll" => list(lines),
        "test" | "t" if tool == "bun" => bun_test(lines),
        "test" if tool == "deno" => deno_test(lines),
        _ => sniff(lines),
    }
}

/// `npm test` / `npm run build` / `pnpm run lint`: recognise the runner by what it printed.
fn sniff(lines: &[String]) -> Option<String> {
    js_test::vitest(lines).or_else(|| js_test::jest(lines)).or_else(|| js_playwright::playwright(lines))
        .or_else(|| if has(lines, r"error TS\d+") { ts_tsc::tsc(lines) } else { None })
        .or_else(|| if has(lines, r"^✖ \d+ problems?") { js_lint::eslint(lines) } else { None })
        .or_else(|| if has(lines, r"^\(pass\)|^\(fail\)") { bun_test(lines) } else { None })
}

fn install(lines: &[String]) -> Option<String> {
    let keep = r"^(added|removed|changed|up to date|audited|found \d+|\d+ packages? (installed|removed)|Packages:|Done in|Lockfile|Already up|Saved|\+ \S|bun (add|install)|Resolved|Installed|npm (error|ERR)|ERR_|error |\s*\d+ (low|moderate|high|critical)|Run `npm audit|To address)|vulnerabilit";
    let mut out = grep(lines, keep);
    out.retain(|l| !l.starts_with("Progress:"));
    let dep = grep(lines, r"(?i)^\s*(npm warn|WARN)\s+deprecated").len();
    let other = grep(lines, r"(?i)^(npm warn|WARN)\b").len() - dep;
    let funding = grep(lines, r"looking for funding").len();
    if out.is_empty() && dep == 0 { return None; }
    let (adds, rest): (Vec<String>, Vec<String>) = out.into_iter().partition(|l| l.starts_with("+ "));
    let mut v = rest;
    if !adds.is_empty() { v.push(format!("+ {}", cap(adds.iter().map(|a| a[2..].to_string()).collect(), 15).join(", "))); }
    if dep > 0 { v.push(format!("{dep} deprecated warning(s)")); }
    if other > 0 { v.push(format!("{other} other warning(s)")); }
    if funding > 0 { v.push("(funding notices dropped)".into()); }
    finish(v)
}

fn outdated(lines: &[String]) -> Option<String> {
    let clean = |l: &str| l.replace(['│', '|', '┃'], " ");
    let rows: Vec<Vec<String>> = lines.iter().filter(|l| !re(r"^[\s─┌└├┐┘┤┬┴┼═╭╰╮╯\-+]*$").is_match(l))
        .map(|l| clean(l).split_whitespace().map(String::from).collect::<Vec<_>>()).filter(|c: &Vec<String>| c.len() >= 3).collect();
    let head = rows.iter().find(|r| r[0] == "Package")?;
    let li = head.iter().position(|c| c == "Latest").unwrap_or(3);
    let out: Vec<String> = rows.iter().filter(|r| r[0] != "Package" && r.len() > li).map(|r| format!("{} {} -> {}", r[0], r[1], r[li])).collect();
    if out.is_empty() { return None; }
    let mut v = vec![format!("{} outdated", out.len())];
    v.extend(cap(out, 40));
    finish(v)
}

fn list(lines: &[String]) -> Option<String> {
    let top = re(r"^[├└]─[─┬] (\S+)");
    let pkgs: Vec<String> = lines.iter().filter_map(|l| top.captures(l).map(|c| c[1].to_string())).collect();
    let mut v: Vec<String> = lines.first().cloned().into_iter().collect();
    if pkgs.is_empty() { return None; }
    v.extend(grep(lines, r"UNMET|invalid|extraneous|missing|ERR"));
    v.push(format!("{} top-level: {}", pkgs.len(), pkgs.join(", ")));
    finish(v)
}

fn bun_test(lines: &[String]) -> Option<String> {
    if !has(lines, r"^\s*\d+ (pass|fail)\b|^Ran \d+ tests?") { return None; }
    let mut out = grep(lines, r"^\(fail\)");
    let start = |l: &str| l.starts_with("error:") || l.contains("AssertionError");
    let stop = |l: &str| l.starts_with("(fail)") || l.starts_with("(pass)") || re(r"^\s*\d+ (pass|fail)|^Ran ").is_match(l);
    out.extend(blocks(lines, &start, &stop, 8, &|l| l.contains("node_modules")));
    out.extend(grep(lines, r"^\s*\d+ (pass|fail|skip|expect)|^Ran \d+"));
    finish(out)
}

fn deno_test(lines: &[String]) -> Option<String> {
    if !has(lines, r"^(ok|FAILED) \|") { return None; }
    let mut out = grep(lines, r"\.\.\. FAILED");
    let start = |l: &str| l.trim() == "ERRORS" || l.trim() == "FAILURES";
    out.extend(blocks(lines, &start, &|l| l.starts_with("ok |") || l.starts_with("FAILED |"), 40, &|_| false));
    out.extend(grep(lines, r"^(ok|FAILED) \|"));
    finish(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::util::t::*;
    fn a(s: &str) -> Vec<String> { s.split_whitespace().map(String::from).collect() }

    #[test]
    fn npm_install_collapses_noise() {
        let mut s = String::new();
        for i in 0..6 { s += &format!("npm warn deprecated pkg{i}@1.0.0: no longer supported\n"); }
        s += "\nadded 312 packages, and audited 313 packages in 9s\n\n58 packages are looking for funding\n  run `npm fund` for details\n\nfound 0 vulnerabilities\n";
        let l = lines(&s);
        let o = small(pkg("npm", &a("install"), &l), &l);
        assert!(o.contains("added 312 packages") && o.contains("6 deprecated") && o.contains("found 0 vulnerabilities"));
        assert!(!o.contains("pkg3@1.0.0") && !o.contains("run `npm fund`"));
    }

    #[test]
    fn pnpm_outdated_table() {
        let s = "┌─────────────┬─────────┬────────┐\n│ Package     │ Current │ Latest │\n├─────────────┼─────────┼────────┤\n│ react       │ 17.0.2  │ 18.3.1 │\n├─────────────┼─────────┼────────┤\n│ typescript  │ 5.3.0   │ 5.6.2  │\n└─────────────┴─────────┴────────┘\n";
        let l = lines(s);
        let o = pkg("pnpm", &a("outdated"), &l).unwrap();
        assert!(o.contains("2 outdated") && o.contains("react 17.0.2 -> 18.3.1") && !o.contains("──"));
        let n = lines("Package  Current  Wanted  Latest  Location  Depended by\nlodash   4.17.0   4.17.21  4.17.21  node_modules/lodash  app\n");
        assert!(pkg("npm", &a("outdated"), &n).unwrap().contains("lodash 4.17.0 -> 4.17.21"));
    }

    #[test]
    fn npm_ls_top_level_only() {
        let l = lines("app@1.0.0 /work/app\n├─┬ express@4.18.2\n│ ├── accepts@1.3.8\n│ └── body-parser@1.20.1\n├── lodash@4.17.21\n└── zod@3.22.0\n");
        let o = list(&l);
        let o = o.unwrap();
        assert!(o.contains("3 top-level: express@4.18.2, lodash@4.17.21, zod@3.22.0") && !o.contains("accepts"));
    }

    #[test]
    fn npm_test_sniffs_vitest() {
        let l = lines("> app@1 test\n> vitest run\n\n ✓ a.test.ts (1 test) 2ms\n\n Test Files  1 passed (1)\n      Tests  1 passed (1)\n   Duration  1s\n");
        let o = pkg("npm", &a("test"), &l).unwrap();
        assert!(o.contains("Test Files  1 passed") && !o.contains("a.test.ts"));
        assert!(pkg("npm", &a("run build"), &lines("hello")).is_none());
    }

    #[test]
    fn bun_and_deno_tests() {
        let l = lines("bun test v1.1.0\n\n(pass) a > ok [0.1ms]\n(pass) a > ok2 [0.1ms]\n(fail) a > bad [0.3ms]\nerror: expect(received).toBe(expected)\n\nExpected: 2\nReceived: 1\n      at /w/a.test.ts:5:3\n\n 2 pass\n 1 fail\nRan 3 tests across 1 files. [20ms]\n");
        let o = bun_test(&l).unwrap();
        assert!(o.contains("(fail) a > bad") && o.contains("Expected: 2") && o.contains("1 fail") && !o.contains("(pass)"));
        let d = lines("running 2 tests from ./a_test.ts\nok_one ... ok (1ms)\nbad_one ... FAILED (2ms)\n\n ERRORS \n\nbad_one => ./a_test.ts:4:6\nerror: AssertionError: Values are not equal\n    at assertEquals (https://deno.land/std/assert.ts:1:1)\n\n FAILURES \n\nbad_one => ./a_test.ts:4:6\n\nFAILED | 1 passed | 1 failed (5ms)\n");
        let o = deno_test(&d).unwrap();
        assert!(o.contains("bad_one ... FAILED") && o.contains("a_test.ts:4:6") && o.contains("FAILED | 1 passed") && !o.contains("ok_one"));
    }
}
