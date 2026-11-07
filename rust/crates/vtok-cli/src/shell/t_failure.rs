use super::failure::failure_view;

fn l(t: &str) -> Vec<String> { t.lines().map(String::from).collect() }

#[test]
fn pytest_failure_keeps_name_place_and_assertion() {
    let py = l("=== FAILURES ===\n____ test_a ____\n    def test_a():\n>       assert 1 == 2\nE       assert 1 == 2\ntest_x.py:3: AssertionError\n=== short test summary info ===\nFAILED test_x.py::test_a - assert 1 == 2\n=== 1 failed, 5 passed in 0.1s ===");
    let v = failure_view(&py).unwrap();
    assert!(v.contains("FAIL test_a") && v.contains("E       assert 1 == 2") && v.contains("at test_x.py:3") && v.contains("1 failed, 5 passed"), "{v}");
    assert!(!v.contains("def test_a"));
}

#[test]
fn parametrized_failures_with_one_assertion_shape_are_folded() {
    let mut many = String::from("=== FAILURES ===\n");
    for i in 0..20 { many += &format!("____ test_p[{i}] ____\n>   assert f({i}) == 1\nE   assert {} == 1\nt.py:5: AssertionError\n", i + 7); }
    many += "=== short test summary info ===\n=== 20 failed, 5 passed in 0.1s ===";
    let v = failure_view(&l(&many)).unwrap();
    assert!(v.matches("FAIL test_p").count() == 1 && v.contains("+19 more failures of test_p") && v.contains("20 failed"), "{v}");
}

#[test]
fn unittest_failure_keeps_frame_and_exception() {
    let ut = l("ERROR: test_k (m.T.test_k)\n----\nTraceback (most recent call last):\n  File \"/t/m.py\", line 9, in test_k\n    {}['k']\nKeyError: 'k'\n\nRan 3 tests in 0.1s\n\nFAILED (errors=1)");
    let v = failure_view(&ut).unwrap();
    assert!(v.contains("ERROR: test_k") && v.contains("m.py:9") && v.contains("KeyError") && v.contains("Ran 3 tests"), "{v}");
}

#[test]
fn cargo_panics_are_shown_with_their_suite_total() {
    let c = l("running 2 tests\ntest a ... ok\ntest b ... FAILED\n\nfailures:\n\n---- b stdout ----\nthread 'b' panicked at src/lib.rs:4:5:\nboom\nnote: run with `RUST_BACKTRACE=1` environment variable to display a backtrace\n\nfailures:\n    b\n\ntest result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s");
    let v = failure_view(&c).unwrap();
    assert!(v.contains("b stdout") && v.contains("boom") && v.contains("1 passed, 1 failed") && !v.contains("RUST_BACKTRACE"), "{v}");
}
