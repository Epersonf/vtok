//! pytest view, real outputs (pytest 9.2, Windows paths): default, -q, green, xdist, -x.
use super::py::pytest;
use super::util::t::lines;

const BENCH: &str = r#"============================= test session starts =============================
platform win32 -- Python 3.12.10, pytest-9.2.0, pluggy-1.7.0
rootdir: C:\Users\epers\AppData\Local\Temp\fx\py
plugins: anyio-4.15.1, platformdirs-4.13.5
collected 40 items

tests\test_a.py .....F...........F...........F..........                 [100%]

================================== FAILURES ===================================
___________________________________ test_5 ____________________________________

    def test_5():
>       assert 5 * 2 == 1
E       assert (5 * 2) == 1

tests\test_a.py:13: AssertionError
___________________________________ test_17 ___________________________________

    def test_17():
>       assert 17 * 2 == 1
E       assert (17 * 2) == 1

tests\test_a.py:37: AssertionError
___________________________________ test_29 ___________________________________

    def test_29():
>       assert 29 * 2 == 1
E       assert (29 * 2) == 1

tests\test_a.py:61: AssertionError
=========================== short test summary info ===========================
FAILED tests/test_a.py::test_5 - assert (5 * 2) == 1
FAILED tests/test_a.py::test_17 - assert (17 * 2) == 1
FAILED tests/test_a.py::test_29 - assert (29 * 2) == 1
======================== 3 failed, 37 passed in 0.13s =========================
"#;

#[test]
fn one_line_per_failure_with_place_and_assertion() {
    let l = lines(BENCH);
    let o = pytest(&l).unwrap();
    assert_eq!(o, "pytest: 3 failed, 37 passed in 0.13s\ntests/test_a.py:13 test_5: assert (5 * 2) == 1\ntests/test_a.py:37 test_17: assert (17 * 2) == 1\ntests/test_a.py:61 test_29: assert (29 * 2) == 1\n");
    assert!(o.len() * 3 < BENCH.len(), "{} vs {}", o.len(), BENCH.len());
}

#[test]
fn quiet_output_has_no_header_and_the_same_view() {
    let q = BENCH.replace("============================= test session starts =============================\nplatform win32 -- Python 3.12.10, pytest-9.2.0, pluggy-1.7.0\nrootdir: C:\\Users\\epers\\AppData\\Local\\Temp\\fx\\py\nplugins: anyio-4.15.1, platformdirs-4.13.5\ncollected 40 items\n\n", "")
        .replace("======================== 3 failed, 37 passed in 0.13s =========================", "3 failed, 37 passed in 0.13s");
    let o = pytest(&lines(&q)).unwrap();
    assert!(o.starts_with("pytest: 3 failed, 37 passed in 0.13s\n") && o.contains("tests/test_a.py:37 test_17: assert (17 * 2) == 1"), "{o}");
}

#[test]
fn green_run_is_one_line() {
    let g = "============================= test session starts =============================\nplatform win32 -- Python 3.12.10\ncollected 31 items\n\ngreen\\test_green.py ..............................s                    [100%]\n\n======================== 30 passed, 1 skipped in 0.04s ========================\n";
    assert_eq!(pytest(&lines(g)).unwrap(), "pytest: 30 passed, 1 skipped in 0.04s\n");
    assert_eq!(pytest(&lines("no tests ran in 0.01s\n")).unwrap(), "pytest: no tests ran in 0.01s\n");
}

#[test]
fn stopping_after_the_first_failure_is_kept() {
    let x = "============================= test session starts =============================\ncollected 19 items\n\ntests\\test_basic.py .F\n\n================================== FAILURES ===================================\n______________________________ test_assert_fail _______________________________\n\n    def test_assert_fail():\n        x = {\"a\": 1}\n>       assert x[\"a\"] == 2\nE       assert 1 == 2\n\ntests\\test_basic.py:12: AssertionError\n=========================== short test summary info ===========================\nFAILED tests/test_basic.py::test_assert_fail - assert 1 == 2\n!!!!!!!!!!!!!!!!!!!!!!!!!! stopping after 1 failures !!!!!!!!!!!!!!!!!!!!!!!!!!\n========================= 1 failed, 1 passed in 0.10s =========================\n";
    let o = pytest(&lines(x)).unwrap();
    assert_eq!(o, "pytest: 1 failed, 1 passed in 0.10s\nstopping after 1 failures\ntests/test_basic.py:12 test_assert_fail: assert 1 == 2\n");
}

#[test]
fn xdist_worker_lines_and_verbose_progress_are_noise() {
    let x = "============================= test session starts ==============================\nplatform linux -- Python 3.11.4, pytest-7.4.0, pluggy-1.2.0 -- /usr/bin/python3\nrootdir: /home/u/proj\nplugins: xdist-3.3.1\ncreated: 2/2 workers\n2 workers [4 items]\n\nscheduling tests via LoadScheduling\n\ntests/test_foo.py::test_one \n[gw0] [ 25%] PASSED tests/test_foo.py::test_one \ntests/test_foo.py::test_two \n[gw1] [ 50%] FAILED tests/test_foo.py::test_two \n\n=================================== FAILURES ===================================\n__________________________________ test_two ____________________________________\n[gw1] linux -- Python 3.11.4 /usr/bin/python3\n\n    def test_two():\n>       assert add(1, 2) == 4\nE       assert 3 == 4\nE        +  where 3 = add(1, 2)\n\ntests/test_foo.py:9: AssertionError\n=========================== short test summary info ============================\nFAILED tests/test_foo.py::test_two - assert 3 == 4\n========================= 1 failed, 3 passed in 0.52s ==========================\n";
    let o = pytest(&lines(x)).unwrap();
    assert_eq!(o, "pytest: 1 failed, 3 passed in 0.52s\ntests/test_foo.py:9 test_two: assert 3 == 4 | +  where 3 = add(1, 2)\n");
}

#[test]
fn not_pytest_output_is_none() {
    assert!(pytest(&lines("hello\nworld")).is_none());
    assert!(pytest(&lines("Ran 3 tests in 0.1s\n\nOK")).is_none());
}
