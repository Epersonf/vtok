//! Built-in declarative filters (`make`, `gcc`, `oxlint`, ...) against the per-tool views: a filter that only truncated must
//! not hide from a view what it groups; a filter that gave a one-line verdict is kept.
use super::compact::preview;

fn compile_errors(n: usize) -> String {
    let mut s = String::from("make[1]: Entering directory '/src'\n");
    for i in 0..n { s += &format!("a.c:{}:5: error: Problem {i:02}\n   12 |   foo(bar_baz_qux);\n      |       ^~~~~~~~~~~\n", i + 1); }
    s + "make: *** [all] Error 1\n"
}

#[test]
fn a_truncating_builtin_filter_does_not_cut_what_the_view_reads() {
    let out = preview("make", &compile_errors(40), 2, 700);
    // the `make` filter keeps 50 lines; 40 errors with their frames are 120 lines
    for i in [0, 17, 39] { assert!(out.contains(&format!("Problem {i:02}")), "error {i} lost:\n{out}"); }
    assert!(out.contains("build FAILED: 40 compile errors"), "{out}");
}

#[test]
fn a_builtin_filter_that_gave_a_verdict_is_kept() {
    let quiet = "make[1]: Entering directory '/src'\n".repeat(40) + "make[1]: Leaving directory '/src'\n";
    assert_eq!(preview("make", &quiet, 0, 700), "make: ok\n");
}
