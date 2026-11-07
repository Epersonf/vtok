use super::run;

const WARN: &str = "warning: unused variable: `x`
 --> src/main.rs:3:9
  |
3 |     let x = 5;
  |         ^ help: if this is intentional, prefix it with an underscore: `_x`
  |
  = note: `#[warn(unused_variables)]` on by default

warning: constant `AGENTS` is never used
 --> src/hook.rs:6:11
  |
6 | pub const AGENTS: &[&str] = &[\"claude\"];
  |           ^^^^^^

warning: `foo` (lib) generated 2 warnings
";

fn test_run(extra: &str) -> String {
    format!("   Compiling foo v0.1.0 (/src/foo)\n{WARN}    Finished `test` profile [unoptimized + debuginfo] target(s) in 1.20s\n     Running unittests src/lib.rs (target/debug/deps/foo-abc)\n\nrunning 3 tests\ntest a::one ... ok\ntest a::two ... ok\ntest a::three ... ok\n{extra}\ntest result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s\n\n     Running unittests src/main.rs (target/debug/deps/foo-def)\n\nrunning 0 tests\n\ntest result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s\n\n   Doc-tests foo\n\nrunning 1 test\ntest src/lib.rs - hello (line 2) ... ok\n\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s\n")
}

#[test]
fn green_test_run_is_one_line_with_counts_and_warning_count() {
    assert_eq!(run("cargo test -p foo", &test_run("")), "cargo test: 4 passed (3 suites), 2 warnings\n");
}

#[test]
fn ignored_tests_and_a_single_suite_are_reported() {
    let s = "running 2 tests\ntest a ... ok\ntest b ... ignored\n\ntest result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.00s\n";
    let input = format!("   Compiling foo v0.1.0\n    Finished `test` profile in 1s\n     Running unittests src/lib.rs\n\n{s}");
    assert_eq!(run("cargo test", &input), "cargo test: 1 passed, 1 ignored\n");
}

#[test]
fn green_build_check_and_doc_are_one_line() {
    let build = format!("   Compiling foo v0.1.0 (/src/foo)\n{WARN}    Finished `release` profile [optimized] target(s) in 22.50s\n");
    assert_eq!(run("cargo build --release", &build), "cargo build: ok, 2 warnings\n");
    let check = "    Checking foo v0.1.0 (/src/foo)\n    Checking bar v0.2.0 (/src/bar)\n    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.52s\n";
    assert_eq!(run("cargo check", check), "cargo check: ok\n");
    let doc = " Documenting foo v0.1.0 (/src/foo)\n    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.52s\n   Generated /src/foo/target/doc/foo/index.html\n";
    assert_eq!(run("cargo doc", doc), "cargo doc: ok\n");
}

#[test]
fn clippy_keeps_its_warnings_because_they_are_the_point() {
    let input = format!("    Checking foo v0.1.0 (/src/foo)\n{WARN}    Finished `dev` profile in 0.5s\n");
    let o = run("cargo clippy", &input);
    assert!(o.contains("warning: unused variable: `x` @ src/main.rs:3:9") && o.contains("AGENTS") && !o.starts_with("cargo clippy: ok"), "{o}");
    let clean = "    Checking foo v0.1.0 (/src/foo)\n    Checking bar v0.2.0 (/src/bar)\n    Finished `dev` profile in 0.5s\n";
    assert_eq!(run("cargo clippy --all-targets", clean), "cargo clippy: ok\n");
}

#[test]
fn nextest_green_run_keeps_the_verdict() {
    let s = "    Starting 3 tests across 1 binary\n        PASS [   0.004s] foo tests::one\n        PASS [   0.005s] foo tests::two\n        PASS [   0.005s] foo tests::three\n------------\n     Summary [   0.010s] 3 tests run: 3 passed, 0 skipped\n";
    assert_eq!(run("cargo nextest run", s), "cargo nextest: 3 passed\n");
    let skipped = s.replace("3 passed, 0 skipped", "3 passed, 2 skipped");
    assert_eq!(run("cargo nextest run", &skipped), "cargo nextest: 3 passed, 2 skipped\n");
}

#[test]
fn printed_test_output_is_counted_not_dropped_silently() {
    let o = run("cargo test", &test_run("hello from test\nanother printed line\n"));
    assert!(o.contains("2 lines of other output omitted"), "{o}");
}

#[test]
fn failures_still_use_the_long_form() {
    let input = "   Compiling foo v0.1.0\n    Finished `test` profile in 1s\n     Running unittests src/lib.rs\n\nrunning 2 tests\ntest a ... ok\ntest b ... FAILED\n\nfailures:\n\n---- b stdout ----\nthread 'b' panicked at src/lib.rs:4:5:\nboom\n\nfailures:\n    b\n\ntest result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s\n\nerror: test failed, to rerun pass `--lib`\n";
    let o = run("cargo test", input);
    assert!(o.contains("b src/lib.rs:4: boom") && o.contains("tests: 1 passed, 1 failed") && !o.starts_with("cargo test: "), "{o}");
}
