use super::clean::clean;
use super::clean_run::{sum_test_results, without_passes};
use super::compact::compact;
use super::listing::summarize_listing;
use super::score::score;
use super::store::valid_id;
use super::summary::summarize;
use vtok_api::ntok;

#[test]
fn strips_ansi_and_progress() {
    let l = clean("\u{1b}[31mred\u{1b}[0m text\nprogress 10%\rprogress 50%\rprogress 100%\n");
    assert_eq!(l, vec!["red text".to_string(), "progress 100%".to_string()]);
}

#[test]
fn collapses_similar_lines_and_keeps_errors() {
    let mut raw = String::new();
    for i in 0..400 { raw.push_str(&format!("compiling crate-{i} v0.{i}.1\n")); }
    raw.push_str("error[E0308]: mismatched types\n  --> src/lib.rs:10:5\n");
    for i in 0..400 { raw.push_str(&format!("step {i} done ok ok ok\n")); }
    raw.push_str("test result: FAILED. 3 passed; 1 failed\n");
    let lines = clean(&raw);
    let s = summarize(&lines, 400);
    assert!(s.contains("mismatched types"), "{s}");
    assert!(s.contains("test result: FAILED"), "{s}");
    assert!(!s.contains("compiling crate-17"), "repeated lines must not be listed one by one: {s}");
    assert!(ntok(&s) <= 400 + 40, "{}", ntok(&s));
}

#[test]
fn zero_counters_are_not_failures() {
    assert_eq!(score("test result: ok. 4 passed; 0 failed; 0 ignored"), 2);
    assert_eq!(score("test result: FAILED. 3 passed; 10 failed"), 3);
    assert_eq!(score("Tests: 5 passed, errors: 0"), 2);
    assert_eq!(score("error[E0308]: mismatched types"), 3);
}

#[test]
fn short_output_is_untouched() {
    assert_eq!(compact("hello\nworld\n", 0, 700), "hello\nworld\n");
}

#[test]
fn ids_are_validated() {
    assert!(valid_id("deadbeef"));
    assert!(!valid_id("../etc/passwd"));
    assert!(!valid_id("abc"));
}

#[test]
fn nonzero_summaries_survive_dedupe() {
    let mut raw = String::new();
    for n in [0, 4, 0, 7, 0] {
        raw.push_str(&format!("   Running unit {n}\ntest result: ok. {n} passed; 0 failed; finished in 0.0{n}s\n\n"));
    }
    let out = summarize(&clean(&raw), 700);
    assert!(out.contains("4 passed") && out.contains("7 passed"), "{out}");
}

#[test]
fn listings_are_grouped_by_directory() {
    let raw: Vec<String> = (0..60).map(|i| format!("./src/mod{}/file{i}.rs", i % 3)).collect();
    let out = summarize_listing(&raw, 300).unwrap();
    assert!(out.contains("60 paths") && out.contains("src/mod0/"), "{out}");
    let hits: Vec<String> = (0..40).map(|i| format!("a/b{}.py:{}:def f{i}():", i % 2, i + 1)).collect();
    assert!(summarize_listing(&hits, 300).unwrap().contains("40 grep matches"));
    let log: Vec<String> = (0..40).map(|i| format!("2026-11-06T10:00:{i:02}Z INFO ok")).collect();
    assert!(summarize_listing(&log, 300).is_none());
}

#[test]
fn test_results_are_summed_and_failures_drop_passing_noise() {
    let l = |t: &[&str]| t.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    let sums = sum_test_results(&l(&["test result: ok. 4 passed; 0 failed; 1 ignored; 0 measured", "test result: ok. 3 passed; 0 failed; 0 ignored"])).unwrap();
    assert!(sums.contains("7 passed") && sums.contains("1 ignored") && sums.contains("2 suites"), "{sums}");
    let mut raw: Vec<String> = (0..10).map(|i| format!("test t{i} ... ok")).collect();
    raw.push("test bad ... FAILED".into());
    let w = without_passes(&raw);
    assert!(w.iter().any(|x| x.contains("10 passing-test lines omitted")) && w.iter().any(|x| x.contains("bad")));
}
