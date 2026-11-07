//! One small hand-written patch per kind of file section, each padded with a filler file so the patch is past the
//! pass-through size and goes through the view.
use super::diff_view;

pub fn l(t: &str) -> Vec<String> { t.lines().map(String::from).collect() }

/// A modified file with 40 changed lines (~500 tokens).
pub fn filler() -> String {
    let mut t = String::from("diff --git a/src/fill.rs b/src/fill.rs\nindex 1..2 100644\n--- a/src/fill.rs\n+++ b/src/fill.rs\n@@ -1,20 +1,20 @@ fn fill() {\n");
    for i in 0..20 { t += &format!("-    let value_{i} = compute_something_old({i}, \"argument\");\n"); }
    for i in 0..20 { t += &format!("+    let value_{i} = compute_something_new({i}, \"argument\");\n"); }
    t
}

fn view(t: &str) -> String { diff_view(&l(&(t.to_string() + &filler())), 700).unwrap() }

#[test]
fn modified_file_keeps_changed_lines_and_drops_boilerplate() {
    // -U25: plenty of context, so keeping one line of it around the change is affordable
    let ctx = |w: &str| (0..25).map(|i| format!(" {w} context line number {i} with words\n")).collect::<String>();
    let d = format!("diff --git a/src/a.rs b/src/a.rs\nindex 1..2 100644\n--- a/src/a.rs\n+++ b/src/a.rs\n@@ -10,51 +10,51 @@ fn alpha() {{\n{}-old line\n+new line\n{}", ctx("before"), ctx("after"));
    let v = diff_view(&l(&d), 700).unwrap();
    assert!(v.contains("M src/a.rs (+1 -1)") && v.contains("@@ -10,51 +10,51 @@ fn alpha() {") && v.contains("-old line\n+new line"), "{v}");
    assert!(v.contains(" before context line number 24 ") && v.contains(" after context line number 0 ") && !v.contains("number 23") && !v.contains("number 1 "), "one line of context: {v}");
    assert!(!v.contains("index 1..2") && !v.contains("+++ b/") && !v.contains("diff --git"), "{v}");
    assert!(v.starts_with("# 1 files +1 -1\n"), "{v}");
    // when the budget is tight (all lines are changes) context goes entirely
    let v = view("diff --git a/b.rs b/b.rs\n--- a/b.rs\n+++ b/b.rs\n@@ -1,3 +1,3 @@\n keep_me_out()\n-x()\n+y()\n");
    assert!(v.contains("@@ -1,3 +1,3 @@\n-x()\n+y()\n") && !v.contains("keep_me_out"), "{v}");
}

#[test]
fn renames_binary_mode_and_deleted() {
    let v = view("diff --git a/old/x.rs b/new/x.rs\nsimilarity index 98%\nrename from old/x.rs\nrename to new/x.rs\n\
diff --git a/img.png b/img.png\nindex 1..2 100644\nBinary files a/img.png and b/img.png differ\n\
diff --git a/run.sh b/run.sh\nold mode 100644\nnew mode 100755\n\
diff --git a/gone.py b/gone.py\ndeleted file mode 100644\nindex 1..0\n--- a/gone.py\n+++ /dev/null\n@@ -1,3 +0,0 @@\n-import os\n-def f():\n-    return 1\n");
    assert!(v.contains("R old/x.rs -> new/x.rs (98%)\n"), "{v}");
    assert!(v.contains("M img.png binary"), "{v}");
    assert!(v.contains("M run.sh mode 100644->100755"), "{v}");
    assert!(v.contains("D gone.py (-3)"), "{v}");
}

#[test]
fn new_file_lock_file_and_generated_paths() {
    let mut nf = String::from("diff --git a/src/n.py b/src/n.py\nnew file mode 100644\nindex 0..1\n--- /dev/null\n+++ b/src/n.py\n@@ -0,0 +1,3 @@\n");
    nf += "+def hello():\n+    return 1\n+X = 2\n";
    let lock = "diff --git a/Cargo.lock b/Cargo.lock\nindex 1..2 100644\n--- a/Cargo.lock\n+++ b/Cargo.lock\n@@ -1,3 +1,3 @@\n [[package]]\n-version = \"1.0.0\"\n+version = \"1.0.1\"\n";
    let min = "diff --git a/web/dist/app.min.js b/web/dist/app.min.js\n--- a/web/dist/app.min.js\n+++ b/web/dist/app.min.js\n@@ -1 +1 @@\n-var a=1\n+var a=2\n";
    let v = view(&format!("{nf}{lock}{min}"));
    assert!(v.contains("A src/n.py (+3)\n+def hello():\n+    return 1\n+X = 2\n"), "{v}");
    assert!(v.contains("M Cargo.lock (+1 -1) generated/lock, not shown") && !v.contains("1.0.1"), "{v}");
    assert!(v.contains("M web/dist/app.min.js (+1 -1) generated/lock, not shown") && !v.contains("var a=2"), "{v}");
}

#[test]
fn crlf_and_whitespace_only_changes_are_flagged_and_collapsed() {
    // the cleaner strips `\r`, so a CRLF -> LF flip reaches the view as identical -/+ pairs
    let crlf = "diff --git a/w.txt b/w.txt\n--- a/w.txt\n+++ b/w.txt\n@@ -1,2 +1,2 @@\n-alpha beta\r\n-gamma\r\n+alpha beta\n+gamma\n";
    let v = diff_view(&crate::shell::clean(&(crlf.to_string() + &filler())), 700).unwrap();
    assert!(v.contains("M w.txt (+2 -2) whitespace/EOL-only, not shown") && !v.contains("gamma"), "{v}");
    let indent = "diff --git a/i.py b/i.py\n--- a/i.py\n+++ b/i.py\n@@ -1,2 +1,2 @@\n-if x:\n-  y()\n+if x:\n+    y()\n";
    assert!(view(indent).contains("M i.py (+2 -2) whitespace/EOL-only"));
    // a block moved inside a hunk is not whitespace-only
    let moved = "diff --git a/m.py b/m.py\n--- a/m.py\n+++ b/m.py\n@@ -1,3 +1,3 @@\n-a()\n b()\n+a()\n";
    assert!(!view(moved).contains("whitespace"));
}

#[test]
fn no_newline_marker_stays_with_its_line() {
    let v = view("diff --git a/e.txt b/e.txt\n--- a/e.txt\n+++ b/e.txt\n@@ -1 +1 @@\n-last\n\\ No newline at end of file\n+last line\n");
    assert!(v.contains("-last\n\\ No newline at end of file\n+last line\n"), "{v}");
    // a newline added at the end and nothing else is an EOL-only change
    let eol = view("diff --git a/f.txt b/f.txt\n--- a/f.txt\n+++ b/f.txt\n@@ -1 +1 @@\n-last\n\\ No newline at end of file\n+last\n");
    assert!(eol.contains("M f.txt (+1 -1) whitespace/EOL-only"), "{eol}");
}

#[test]
fn combined_merge_diff() {
    let v = view("diff --cc src/m.rs\nindex 1,2..3\n--- a/src/m.rs\n+++ b/src/m.rs\n@@@ -1,4 -1,3 +1,4 @@@ fn merged() {\n  keep\n- from_ours\n +from_theirs\n++both_new\n  tail\n");
    assert!(v.contains("M src/m.rs (+2 -1) merge") && v.contains("@@@ -1,4 -1,3 +1,4 @@@ fn merged() {"), "{v}");
    assert!(v.contains("- from_ours") && v.contains(" +from_theirs") && v.contains("++both_new"), "{v}");
}

#[test]
fn long_lines_are_cut_and_say_so() {
    let long = "x".repeat(500);
    let v = view(&format!("diff --git a/l.txt b/l.txt\n--- a/l.txt\n+++ b/l.txt\n@@ -1 +1 @@\n-short\n+{long}\n"));
    assert!(v.contains(&format!("+{}…[+341 chars]", "x".repeat(159))), "{v}");
}

#[test]
fn identical_hunks_are_collapsed() {
    let h = "@@ -5,2 +5,2 @@ fn cfg() {\n-    let timeout_seconds = 30;\n-    let retries = 3;\n+    let timeout_seconds = 60;\n+    let retries = 5;\n";
    let v = view(&format!("diff --git a/a.rs b/a.rs\n--- a/a.rs\n+++ b/a.rs\n{h}diff --git a/b.rs b/b.rs\n--- a/b.rs\n+++ b/b.rs\n{h}"));
    assert_eq!(v.matches("timeout_seconds = 60").count(), 1, "{v}");
    assert!(v.contains("(same 4 changed lines as a.rs @@ -5,2 +5,2)"), "{v}");
}
