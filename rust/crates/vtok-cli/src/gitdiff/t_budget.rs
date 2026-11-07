//! Big patches: the file list is complete, the budget holds, small hand-written changes survive whole while the
//! biggest file is cut first, new files get a preview plus their symbols, and cuts are announced.
use super::t_kinds::l;
use super::{budget_for, diff_view};
use vtok_api::ntok;

fn modified(name: &str, n: usize) -> String {
    let mut t = format!("diff --git a/{name} b/{name}\nindex 1..2 100644\n--- a/{name}\n+++ b/{name}\n@@ -1,{n} +1,{n} @@ fn body() {{\n");
    for i in 0..n { t += &format!("-    let item_{i} = old_call(\"{name}\", {i});\n+    let item_{i} = new_call(\"{name}\", {i}, true);\n"); }
    t
}

fn big() -> String {
    let mut t = String::new();
    t += &modified("src/small.rs", 3);
    t += &modified("src/huge.rs", 900);
    for i in 0..15 { t += &modified(&format!("src/mid{i}.rs"), 6); }
    t += "diff --git a/src/fresh.rs b/src/fresh.rs\nnew file mode 100644\n--- /dev/null\n+++ b/src/fresh.rs\n@@ -0,0 +1,120 @@\n";
    for i in 0..40 { t += &format!("+pub fn helper_{i}(x: u32) -> u32 {{\n+    x + {i}\n+}}\n"); }
    t += "diff --git a/src/old.rs b/src/old.rs\ndeleted file mode 100644\n--- a/src/old.rs\n+++ /dev/null\n@@ -1,300 +0,0 @@\n";
    for i in 0..300 { t += &format!("-fn legacy_{i}() {{ do_old_thing({i}); }}\n"); }
    t
}

#[test]
fn budget_scales_and_is_bounded() {
    // below ~5000 tokens the budget exceeds the patch itself: nothing is cut
    assert_eq!(budget_for(500, 700), 2750);
    assert_eq!(budget_for(4000, 700), 4500);
    assert_eq!(budget_for(6000, 700), 5500);
    assert_eq!(budget_for(100_000, 700), 8000);
    assert_eq!(budget_for(100_000, 9000), 9000);
}

#[test]
fn big_patch_keeps_every_file_and_prioritises() {
    let raw = big();
    let v = diff_view(&l(&raw), 700).unwrap();
    let b = budget_for(ntok(&raw), 700);
    assert!(ntok(&v) <= b + 30 && ntok(&v) > b * 9 / 10, "{} vs {b}", ntok(&v));
    for i in 0..15 { assert!(v.contains(&format!("M src/mid{i}.rs (+6 -6)")), "mid{i} missing"); }
    assert!(v.contains("M src/huge.rs (+900 -900)") && v.contains("A src/fresh.rs (+120)") && v.contains("D src/old.rs (-300)\n"), "{v}");
    // small and medium files whole, the huge one cut with a note
    assert!(v.contains("+    let item_2 = new_call(\"src/small.rs\", 2, true);") && v.contains("+    let item_5 = new_call(\"src/mid14.rs\", 5, true);"), "{v}");
    assert!(v.contains("more changed lines") && !v.contains("item_899 = new_call(\"src/huge.rs\""), "{v}");
    // the new file: first lines, then its symbols from the outline
    assert!(v.contains("+pub fn helper_0(x: u32) -> u32 {") && v.contains("symbols: ") && v.contains("helper_39"), "{v}");
    // the deleted file is one line; its content is not shown
    assert!(!v.contains("legacy_7"), "{v}");
    assert!(v.contains("vtok output ID --file PATH"), "{v}");
}

#[test]
fn many_files_still_listed_when_headers_exceed_the_budget() {
    let mut t = String::new();
    for i in 0..400 { t += &modified(&format!("pkg/very/long/directory/name/file_{i}.rs"), 2); }
    let v = diff_view(&l(&t), 700).unwrap();
    assert_eq!(v.matches("\nM pkg/very/long/directory/name/file_").count(), 400);
}

#[test]
fn medium_patch_is_whole_without_boilerplate() {
    let mut t = String::new();
    for i in 0..6 { t += &modified(&format!("src/m{i}.rs"), 8); }
    let v = diff_view(&l(&t), 700).unwrap();
    assert!(!v.contains("more changed lines") && !v.contains("--file PATH"), "{v}");
    for i in 0..6 { for j in 0..8 { assert!(v.contains(&format!("+    let item_{j} = new_call(\"src/m{i}.rs\", {j}, true);"))); } }
    assert!(ntok(&v) * 100 / ntok(&t) < 95,"{} vs {}", ntok(&v), ntok(&t));
}
