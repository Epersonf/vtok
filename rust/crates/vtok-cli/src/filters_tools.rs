//! More per-tool views, chosen by output shape like `filters`: eslint (stylish), jest/vitest, npm/pip installs, cargo
//! builds, terraform plans, go test and Maven/Gradle. Each returns None unless the output clearly is that tool's.
use crate::filters::{render_diags, Diag};

fn count_prefix(lines: &[String], pre: &[&str]) -> usize { lines.iter().filter(|l| pre.iter().any(|p| l.trim_start().starts_with(p))).count() }
fn tail_with(lines: &[String], pre: &[&str]) -> Vec<String> { lines.iter().filter(|l| pre.iter().any(|p| l.trim_start().starts_with(p))).cloned().collect() }

/// eslint's default "stylish" format: a path line, then `  12:5  error  message  rule` lines.
pub fn eslint(lines: &[String]) -> Option<String> {
    let mut ds = vec![];
    let mut file = String::new();
    for l in lines {
        if !l.starts_with(' ') && !l.trim().is_empty() && !l.starts_with('✖') && (l.contains('/') || l.contains('\\') || l.contains('.')) && !l.contains("  ") { file = l.trim().to_string(); continue; }
        let t = l.trim();
        let mut it = t.split_whitespace();
        let (Some(pos), Some(sev)) = (it.next(), it.next()) else { continue };
        let Some((ln, col)) = pos.split_once(':') else { continue };
        if !(ln.bytes().all(|b| b.is_ascii_digit()) && col.bytes().all(|b| b.is_ascii_digit()) && !ln.is_empty()) || !matches!(sev, "error" | "warning") || file.is_empty() { continue; }
        let rest: Vec<&str> = it.collect();
        let rule = rest.last().filter(|r| r.contains('-') || r.contains('/') || r.chars().all(|c| c.is_ascii_lowercase())).map(|r| r.to_string()).unwrap_or_else(|| sev.to_string());
        let msg = rest[..rest.len().saturating_sub(1)].join(" ");
        ds.push(Diag { file: file.clone(), line: ln.to_string(), code: format!("{sev}:{rule}"), msg });
    }
    if ds.len() < 6 { return None; }
    Some(render_diags(&ds, lines))
}

/// jest / vitest: failures with their first lines, then the summary; passing noise is dropped.
pub fn jest(lines: &[String]) -> Option<String> {
    let has_summary = lines.iter().any(|l| l.starts_with("Tests:") || l.trim_start().starts_with("Tests ") || l.starts_with("Test Suites:") || l.trim_start().starts_with("Test Files"));
    if !has_summary { return None; }
    let mut out = vec![];
    let mut keep = 0usize;
    for l in lines {
        let t = l.trim_start();
        if t.starts_with('●') || t.starts_with("FAIL ") || t.starts_with("× ") || t.starts_with("✗ ") || t.starts_with("❯ ") { out.push(l.trim_end().to_string()); keep = 7; continue; }
        if keep > 0 && !t.is_empty() { out.push(format!("  {}", l.trim())); keep -= 1; continue; }
        if t.starts_with("Tests:") || t.starts_with("Test Suites:") || t.starts_with("Tests ") || t.starts_with("Test Files") || t.starts_with("Time:") || t.starts_with("Duration") { out.push(l.trim().to_string()); }
    }
    (!out.is_empty()).then(|| out.join("\n") + "\n")
}

/// npm / pnpm / yarn / pip installs: the result line, vulnerabilities and errors; progress and deprecation chatter folded.
pub fn install(lines: &[String]) -> Option<String> {
    let npm = lines.iter().any(|l| l.starts_with("added ") && l.contains("packages") || l.contains("up to date, audited") || l.starts_with("npm warn") || l.starts_with("npm WARN"));
    let pip = lines.iter().any(|l| l.starts_with("Successfully installed") || l.starts_with("Collecting ") || l.starts_with("Requirement already satisfied"));
    if !npm && !pip { return None; }
    let warns = count_prefix(lines, &["npm warn", "npm WARN", "WARNING:"]);
    let mut out = tail_with(lines, &["added ", "removed ", "changed ", "up to date", "Successfully installed", "found ", "npm error", "npm ERR", "ERROR:", "error:", "Done in"]);
    let extra: Vec<String> = lines.iter().filter(|l| l.contains("vulnerabilit") && !out.contains(l)).cloned().collect();
    out.extend(extra);
    let skipped = count_prefix(lines, &["Requirement already satisfied", "Collecting ", "Downloading ", "Using cached "]);
    if skipped > 0 { out.insert(0, format!("({skipped} download/collect lines hidden)")); }
    if warns > 0 { out.push(format!("({warns} warnings hidden)")); }
    (out.len() >= 1).then(|| out.join("\n") + "\n")
}

/// cargo: crates compiled/downloaded collapsed to counts; warnings and errors stay.
pub fn cargo(lines: &[String]) -> Option<String> {
    let progress = ["Compiling ", "Downloading ", "Downloaded ", "Fresh ", "Locking ", "Updating ", "Adding ", "Checking ", "Blocking ", "Unpacking "];
    let n = count_prefix(lines, &progress);
    if n < 5 { return None; }
    let rest: Vec<&String> = lines.iter().filter(|l| !progress.iter().any(|p| l.trim_start().starts_with(p))).collect();
    let mut out = vec![format!("({n} crates compiled/fetched)")];
    out.extend(rest.iter().map(|l| l.to_string()));
    Some(out.join("\n") + "\n")
}

/// terraform plan: the resources that change and the totals.
pub fn terraform(lines: &[String]) -> Option<String> {
    if !lines.iter().any(|l| l.starts_with("Plan:") || l.contains("No changes.") || l.contains("Terraform will perform")) { return None; }
    let out = tail_with(lines, &["# ", "Plan:", "No changes", "Error", "│ Error", "Apply complete", "Changes to Outputs"]);
    (!out.is_empty()).then(|| out.join("\n") + "\n")
}

/// go test: passing packages counted, failures and panics shown.
pub fn go_test(lines: &[String]) -> Option<String> {
    let ok = lines.iter().filter(|l| l.starts_with("ok  \t") || l.starts_with("ok \t")).count();
    if ok < 4 { return None; }
    let mut out = vec![format!("ok: {ok} packages")];
    out.extend(lines.iter().filter(|l| !(l.starts_with("ok  \t") || l.starts_with("ok \t") || l.starts_with("?   \t") || l.starts_with("=== RUN") || l.starts_with("=== PAUSE") || l.starts_with("=== CONT") || l.contains("--- PASS"))).cloned());
    Some(out.join("\n") + "\n")
}

/// Maven / Gradle: [INFO] chatter dropped, [ERROR]/[WARNING] and the build result kept.
pub fn jvm(lines: &[String]) -> Option<String> {
    if count_prefix(lines, &["[INFO]"]) < 8 { return None; }
    let out: Vec<String> = lines.iter().filter(|l| { let t = l.trim_start(); !t.starts_with("[INFO]") || t.contains("BUILD") || t.contains("Tests run:") || t.contains("Total time") || t.contains("Reactor Summary") }).cloned().collect();
    Some(out.join("\n") + "\n")
}

pub fn view(lines: &[String]) -> Option<String> {
    eslint(lines).or_else(|| jest(lines)).or_else(|| terraform(lines)).or_else(|| jvm(lines)).or_else(|| go_test(lines)).or_else(|| cargo(lines)).or_else(|| install(lines))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn l(s: &str) -> Vec<String> { s.lines().map(String::from).collect() }

    #[test]
    fn eslint_groups_by_rule() {
        let mut s = String::from("/p/src/a.js\n");
        for i in 1..=7 { s += &format!("  {i}:5  error  'x' is defined but never used  no-unused-vars\n"); }
        s += "\n✖ 7 problems (7 errors, 0 warnings)\n";
        let v = eslint(&l(&s)).unwrap();
        assert!(v.contains("error:no-unused-vars x7") && v.contains("/p/src/a.js:1,2,3,4,5,6,7") && v.contains("7 problems"), "{v}");
    }

    #[test]
    fn jest_keeps_failures_and_summary() {
        let s = "PASS a.test.js\nPASS b.test.js\nFAIL c.test.js\n  ● adds › works\n\n    expect(received).toBe(expected)\n    Expected: 3\n    Received: 2\n\nTests:       1 failed, 9 passed, 10 total\nTime:        1.2 s\n";
        let v = jest(&l(s)).unwrap();
        assert!(v.contains("FAIL c.test.js") && v.contains("Expected: 3") && v.contains("Tests:") && !v.contains("PASS a"), "{v}");
    }

    #[test]
    fn installs_builds_and_plans_fold() {
        let npm = "npm warn deprecated a@1\nnpm warn deprecated b@1\nadded 120 packages, and audited 121 packages in 4s\n\nfound 0 vulnerabilities\n";
        let v = install(&l(npm)).unwrap();
        assert!(v.contains("added 120 packages") && v.contains("(2 warnings hidden)") && !v.contains("deprecated"), "{v}");
        let c = "   Compiling a v1\n   Compiling b v1\n   Compiling c v1\n   Compiling d v1\n   Compiling e v1\nwarning: unused variable\n    Finished dev in 3s\n";
        assert!(cargo(&l(c)).unwrap().starts_with("(5 crates compiled/fetched)") && cargo(&l(c)).unwrap().contains("warning: unused"));
        let t = "Refreshing state...\n  # aws_s3_bucket.a will be created\n  + resource x\nPlan: 1 to add, 0 to change, 0 to destroy.\n";
        let v = terraform(&l(t)).unwrap();
        assert!(v.contains("# aws_s3_bucket.a will be created") && v.contains("Plan: 1 to add") && !v.contains("Refreshing"), "{v}");
        let g = "ok  \tpkg/a\t0.1s\nok  \tpkg/b\t0.1s\nok  \tpkg/c\t0.1s\nok  \tpkg/d\t0.1s\n--- FAIL: TestX (0.00s)\nFAIL\tpkg/e\t0.2s\n";
        let v = go_test(&l(g)).unwrap();
        assert!(v.starts_with("ok: 4 packages") && v.contains("--- FAIL: TestX"), "{v}");
    }
}
