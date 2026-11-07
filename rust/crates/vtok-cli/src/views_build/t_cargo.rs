use super::{run, view, ls};

const BUILD: &str = "   Compiling libc v0.2.150
   Compiling serde v1.0.190
   Compiling foo v0.1.0 (/src/foo)
warning: unused variable: `x`
 --> src/main.rs:3:9
  |
3 |     let x = 5;
  |         ^ help: if this is intentional, prefix it with an underscore: `_x`
  |
  = note: `#[warn(unused_variables)]` on by default

warning: unused variable: `y`
 --> src/lib.rs:9:9
  |
9 |     let y = 5;
  |         ^

error[E0308]: mismatched types
  --> src/main.rs:10:18
   |
10 |     let n: i32 = \"a\";
   |            ---   ^^^ expected `i32`, found `&str`
   |            |
   |            expected due to this

warning: `foo` (bin \"foo\") generated 2 warnings
error: could not compile `foo` (bin \"foo\") due to 1 previous error; 2 warnings emitted
";

#[test]
fn build_groups_warnings_and_keeps_errors() {
    let o = run("cd app && cargo build --release", BUILD);
    assert!(o.contains("error[E0308]: mismatched types @ src/main.rs:10:18"), "{o}");
    assert!(o.contains("warning: unused variable: `x` @ src/main.rs:3:9"));
    assert!(o.contains("could not compile `foo`"));
    assert!(!o.contains("Compiling libc") && !o.contains("expected due to this") && !o.contains("generated 2"));
}

const TEST: &str = "   Compiling foo v0.1.0 (/src/foo)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 1.20s
     Running unittests src/lib.rs (target/debug/deps/foo-abc)

running 3 tests
test a::ok_one ... ok
test a::ok_two ... ok
test a::bad ... FAILED

failures:

---- a::bad stdout ----
thread 'a::bad' panicked at src/a.rs:20:9:
assertion `left == right` failed
  left: 1
 right: 2
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    a::bad

test result: FAILED. 2 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests foo

running 1 test
test src/lib.rs - hello (line 2) ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s

error: test failed, to rerun pass `--lib`
";

#[test]
fn test_keeps_failure_block_and_sums_results() {
    let o = run("cargo test", TEST);
    assert!(o.contains("---- a::bad stdout ----") && o.contains("panicked at src/a.rs:20:9"));
    assert!(o.contains("left: 1"));
    assert!(o.contains("tests: 3 passed, 1 failed, 0 ignored (2 suites)"), "{o}");
    assert!(o.contains("error: test failed"));
    assert!(!o.contains("ok_one") && !o.contains("RUST_BACKTRACE") && !o.contains("Doc-tests"));
}

const NEXTEST: &str = "    Starting 3 tests across 1 binary
        PASS [   0.004s] foo tests::one
        PASS [   0.005s] foo tests::two
        FAIL [   0.006s] foo tests::three

--- STDOUT:              foo tests::three ---
running 1 test
test tests::three ... FAILED

--- STDERR:              foo tests::three ---
thread 'tests::three' panicked at src/lib.rs:7:5:
boom
------------
     Summary [   0.010s] 3 tests run: 2 passed, 1 failed, 0 skipped
        FAIL [   0.006s] foo tests::three
";

#[test]
fn nextest_keeps_failures_and_summary() {
    let o = run("cargo nextest run", NEXTEST);
    assert!(o.contains("FAIL [   0.006s] foo tests::three") && o.contains("boom"));
    assert!(o.contains("3 tests run: 2 passed, 1 failed"));
    assert!(!o.contains("tests::one"));
    assert_eq!(o.matches("FAIL [").count(), 1, "{o}");
}

#[test]
fn bench_keeps_timings_only() {
    let s = "   Compiling foo v0.1.0\n    Finished `bench` profile in 3s\n     Running unittests src/lib.rs\n\nrunning 2 tests\ntest sort_small ... bench:       1,234 ns/iter (+/- 12)\ntest sort_big   ... bench:     987,000 ns/iter (+/- 900)\n\ntest result: ok. 0 passed; 0 failed; 0 ignored; 2 measured; 0 filtered out; finished in 1.0s\n";
    let o = run("cargo bench", s);
    assert!(o.contains("sort_small ... bench:") && o.contains("sort_big"));
}

#[test]
fn not_cargo_output_is_none() {
    assert!(view("cargo build", &ls("hello\nworld")).is_none());
    assert!(view("echo hi", &ls("Compiling x")).is_none());
}
