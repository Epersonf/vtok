//! Command variants: show/log -p commit headers, colour, -U0, --word-diff, --stat, --name-status, small patches,
//! and `vtok output ID --file PATH`'s section lookup.
use super::t_kinds::{filler, l};
use super::{diff_view, section::file_sections};

const SHA: &str = "0123456789abcdef0123456789abcdef01234567";

#[test]
fn small_patch_passes_through_unchanged() {
    let d = "diff --git a/a.rs b/a.rs\nindex 1..2 100644\n--- a/a.rs\n+++ b/a.rs\n@@ -1 +1 @@\n-a\n+b";
    assert_eq!(diff_view(&l(d), 700).unwrap(), format!("{d}\n"));
}

#[test]
fn show_and_log_p_keep_commit_headers_between_files() {
    let mut t = format!("commit {SHA}\nAuthor: Ana Silva <ana@x.io>\nDate:   Fri Nov 6 12:45:00 2026 +0000\n\n    Fix the parser\n\n    Body line.\n\n");
    t += &filler();
    t += &format!("\ncommit {}\nAuthor: Bo <b@x.io>\nDate:   Thu Nov 5 09:00:00 2026 +0000\n\n    Second change\n\n", SHA.replace('0', "f"));
    t += "diff --git a/z.rs b/z.rs\n--- a/z.rs\n+++ b/z.rs\n@@ -1,2 +1,2 @@\n-gone_line()\n+kept_line()\n \n";
    let v = diff_view(&l(&t), 700).unwrap();
    assert!(v.contains("commit 0123456789\nAuthor: Ana Silva\nDate: Fri Nov 6 12:45:00 2026 +0000\n  Fix the parser\n  Body line.\nM src/fill.rs"), "{v}");
    assert!(v.contains("commit f123456789\nAuthor: Bo") && v.contains("  Second change\nM z.rs (+1 -1)") && v.contains("+kept_line()"), "{v}");
    assert!(!v.contains("ana@x.io"), "{v}");
}

#[test]
fn indentation_survives_the_pipeline() {
    // padded `+    code` lines must not be taken for a column-aligned table and squeezed
    let out = crate::shell::preview("git diff", &filler(), 0, 700);
    assert!(out.contains("+    let value_0 = compute_something_new(0, \"argument\");"), "{out}");
}

#[test]
fn colour_and_zero_context() {
    let raw = format!("\u{1b}[1mdiff --git a/c.rs b/c.rs\u{1b}[m\n\u{1b}[1m--- a/c.rs\u{1b}[m\n\u{1b}[1m+++ b/c.rs\u{1b}[m\n\u{1b}[36m@@ -3 +3 @@ fn c()\u{1b}[m\n\u{1b}[31m-red()\u{1b}[m\n\u{1b}[32m+green()\u{1b}[m\n{}", filler());
    let v = diff_view(&crate::shell::clean(&raw), 700).unwrap();
    assert!(v.contains("M c.rs (+1 -1)\n@@ -3 +3 @@ fn c()\n-red()\n+green()\n"), "{v}");
}

#[test]
fn word_diff_lines_are_kept() {
    let t = format!("diff --git a/w.rs b/w.rs\n--- a/w.rs\n+++ b/w.rs\n@@ -1,3 +1,3 @@ fn w() {{\n    same();\n    [-old_name-]{{+new_name+}}(x);\n    also_same();\n{}", filler());
    let v = diff_view(&l(&t), 700).unwrap();
    assert!(v.contains("M w.rs (+1 -1)") && v.contains("    [-old_name-]{+new_name+}(x);"), "{v}");
}

#[test]
fn stat_and_name_status_pass_through() {
    let mut s = String::new();
    for i in 0..40 { s += &format!(" rust/crates/vtok-cli/src/module_{i}.rs | {} +++--\n", i + 3); }
    s += " 40 files changed, 300 insertions(+), 120 deletions(-)";
    assert_eq!(diff_view(&l(&s), 700).unwrap(), format!("{s}\n"));
    let ns: String = (0..60).map(|i| format!("{}\tsrc/dir/file_{i}.rs\n", ["M", "A", "D", "R100"][i % 4])).collect();
    assert_eq!(diff_view(&l(&ns), 700).unwrap(), ns);
    assert!(diff_view(&l("just\nsome\ntext"), 700).is_none());
}

#[test]
fn file_section_lookup() {
    let t = format!("commit {SHA}\n\n    msg\n\n{}diff --git a/src/b.rs b/src/b.rs\n--- a/src/b.rs\n+++ b/src/b.rs\n@@ -1 +1 @@\n-x\n+y\n", filler());
    let lines = l(&t);
    let s = file_sections(&lines, "src/b.rs").unwrap();
    assert!(s.starts_with("# lines 50-55\ndiff --git a/src/b.rs b/src/b.rs\n") && s.ends_with("-x\n+y\n"), "{s}");
    assert!(file_sections(&lines, "fill.rs").unwrap().contains("compute_something_new(19"));
    assert!(file_sections(&lines, "nope.rs").is_err());
}
