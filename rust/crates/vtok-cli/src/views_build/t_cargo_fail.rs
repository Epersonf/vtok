//! A failing libtest test is one line, `name file:line message`, unless its output says more than that.
use super::run;

fn report(blocks: &str) -> String {
    format!("running 3 tests\ntest t::a ... FAILED\ntest t::b ... FAILED\ntest t::c ... ok\n\nfailures:\n{blocks}\nfailures:\n    t::a\n    t::b\n\ntest result: FAILED. 1 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s\n\nerror: test failed, to rerun pass `--lib`\n")
}

#[test]
fn assertion_and_plain_panics_are_one_line_each() {
    let o = run("cargo test", &report("\n---- t::a stdout ----\n\nthread 't::a' (12604) panicked at src\\lib.rs:23:19:\nassertion `left == right` failed\n  left: 38\n right: 1\nnote: run with `RUST_BACKTRACE=1` environment variable to display a backtrace\n\n---- t::b stdout ----\n\nthread 't::b' (8140) panicked at src/lib.rs:37:9:\nindex out of bounds: the len is 3 but the index is 7\n\n"));
    assert!(o.contains("t::a src\\lib.rs:23: assertion `left == right` failed left: 38 right: 1\n"), "{o}");
    assert!(o.contains("t::b src/lib.rs:37: index out of bounds: the len is 3 but the index is 7\n"), "{o}");
    assert!(!o.contains("---- t::a") && !o.contains("thread 't::a'"), "{o}");
    assert!(o.contains("tests: 1 passed, 2 failed, 0 ignored (1 suites)"), "{o}");
}

#[test]
fn a_test_that_printed_or_a_long_message_keeps_its_block() {
    let o = run("cargo test", &report("\n---- t::a stdout ----\nsetting up\nloaded 3 rows\nthread 't::a' panicked at src/lib.rs:5:5:\nboom\n\n---- t::b stdout ----\nthread 't::b' panicked at src/lib.rs:9:5:\nline one\nline two\nline three\nline four\nline five\n\n"));
    assert!(o.contains("---- t::a stdout ----") && o.contains("loaded 3 rows") && o.contains("boom"), "{o}");
    assert!(o.contains("---- t::b stdout ----") && o.contains("line four"), "{o}");
}
