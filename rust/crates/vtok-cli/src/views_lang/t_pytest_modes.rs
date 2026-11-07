//! pytest view: --tb modes, setup/teardown/collection errors, -r summaries, parametrize folding, captured output.
use super::py::pytest;
use super::util::t::lines;

const TAIL: &str = "============================== warnings summary ===============================\ntests/test_basic.py::test_warn\n  C:\\p\\tests\\test_basic.py:61: DeprecationWarning: careful\n    warnings.warn(\"careful\", DeprecationWarning)\n\n-- Docs: https://docs.pytest.org/en/stable/how-to/capture-warnings.html\n";

fn run(body: &str, summary: &str) -> String { pytest(&lines(&format!("{body}{TAIL}{summary}"))).unwrap() }

#[test]
fn tb_short_puts_the_frame_before_the_error_lines() {
    let body = "=== test session starts ===\nrootdir: C:\\p\ncollected 2 items\n\n=================================== FAILURES ===================================\n______________________________ test_param[2] ________________________________\ntests\\test_basic.py:27: in test_param\n    assert n % 2 == 1, f\"even {n}\"\nE   AssertionError: even 2\nE   assert (2 % 2) == 1\n";
    let o = run(body, "=========================== short test summary info ===========================\nFAILED tests/test_basic.py::test_param[2] - AssertionError: even 2\n== 1 failed, 1 passed, 1 warning in 0.13s ==\n");
    assert_eq!(o, "pytest: 1 failed, 1 passed, 1 warning in 0.13s\ntests/test_basic.py:27 test_param[2]: AssertionError: even 2 | assert (2 % 2) == 1\n");
}

#[test]
fn tb_line_pairs_location_lines_with_the_summary_by_message() {
    let body = "=================================== FAILURES ===================================\nE   assert 1 == 2\nC:\\p\\tests\\test_basic.py:12: assert 1 == 2\nE   AssertionError: even 2\n    assert (2 % 2) == 1\nC:\\p\\tests\\test_basic.py:27: AssertionError: even 2\n[XPASS(strict)] \n[XPASS(strict)] \n";
    let o = run(body, "=========================== short test summary info ===========================\nFAILED tests/test_basic.py::test_assert_fail - assert 1 == 2\nFAILED tests/test_basic.py::test_param[2] - AssertionError: even 2\nFAILED tests/test_basic.py::test_xpass_strict - [XPASS(strict)]\n== 3 failed in 0.04s ==\n");
    assert_eq!(o, "pytest: 3 failed in 0.04s\ntests/test_basic.py:12 test_assert_fail: assert 1 == 2\ntests/test_basic.py:27 test_param[2]: AssertionError: even 2\ntests/test_basic.py test_xpass_strict: [XPASS(strict)]\n");
}

#[test]
fn tb_no_has_only_the_summary_lines() {
    let o = pytest(&lines("collected 3 items\n\nt.py .FF\n\n=========================== short test summary info ===========================\nFAILED tests/t.py::test_a - assert 1 == 2\nFAILED tests/t.py::TestK::test_b - KeyError: 'x'\n== 2 failed, 1 passed in 0.04s ==\n")).unwrap();
    assert_eq!(o, "pytest: 2 failed, 1 passed in 0.04s\ntests/t.py test_a: assert 1 == 2\ntests/t.py TestK::test_b: KeyError: 'x'\n");
}

#[test]
fn setup_teardown_and_collection_errors_say_which_phase() {
    let body = "=================================== ERRORS ====================================\n____________________ ERROR at setup of test_uses_bad_setup ____________________\n\n    @pytest.fixture\n    def bad_setup():\n>       raise RuntimeError(\"setup exploded\")\nE       RuntimeError: setup exploded\n\ntests\\test_fixture_errors.py:6: RuntimeError\n_________________ ERROR at teardown of test_uses_bad_teardown _________________\nE       RuntimeError: teardown exploded\n\ntests\\test_fixture_errors.py:12: RuntimeError\n_______________ ERROR collecting collect/test_collect_error.py ________________\nImportError while importing test module 'C:\\p\\collect\\test_collect_error.py'.\nTraceback:\n..\\..\\Lib\\importlib\\__init__.py:90: in import_module\n    return _bootstrap._gcd_import(name[level:], package, level)\ncollect\\test_collect_error.py:1: in <module>\n    import nonexistent_module_xyz\nE   ModuleNotFoundError: No module named 'nonexistent_module_xyz'\n";
    let o = run(body, "=========================== short test summary info ===========================\nERROR tests/test_fixture_errors.py::test_uses_bad_setup - RuntimeError: setup...\nERROR tests/test_fixture_errors.py::test_uses_bad_teardown - RuntimeError: te...\nERROR collect/test_collect_error.py\n!!!!!!!!!!!!!!!!!!! Interrupted: 1 error during collection !!!!!!!!!!!!!!!!!!!!\n== 1 passed, 3 errors in 0.11s ==\n");
    assert_eq!(o, "pytest: 1 passed, 3 errors in 0.11s\nInterrupted: 1 error during collection\nERROR setup tests/test_fixture_errors.py:6 test_uses_bad_setup: RuntimeError: setup exploded\nERROR teardown tests/test_fixture_errors.py:12 test_uses_bad_teardown: RuntimeError: teardown exploded\nERROR collect collect/test_collect_error.py:1 ModuleNotFoundError: No module named 'nonexistent_module_xyz'\n");
}

#[test]
fn syntax_error_reports_the_exception_line_and_its_place() {
    let body = "=== ERRORS ===\n________________ ERROR collecting collect/test_syntax_error.py ________________\n..\\..\\Lib\\ast.py:52: in parse\n    return compile(source, filename, mode, flags,\nE     File \"C:\\p\\collect\\test_syntax_error.py\", line 1\nE       def test_y(:\nE                  ^\nE   SyntaxError: invalid syntax\n";
    let o = pytest(&lines(&format!("rootdir: C:\\p\n{body}=== short test summary info ===\nERROR collect/test_syntax_error.py\n== 1 error in 0.22s ==\n"))).unwrap();
    assert!(o.ends_with("ERROR collect collect/test_syntax_error.py:1 SyntaxError: invalid syntax\n"), "{o}");
}

#[test]
fn skips_are_grouped_by_reason_and_xfail_xpass_listed() {
    let o = pytest(&lines("=========================== short test summary info ===========================\nSKIPPED [2] tests\\a.py:30: not ready\nSKIPPED [1] tests\\a.py:35: needs gpu\nSKIPPED [1] tests\\b.py:5: not ready\nXFAIL tests/a.py::test_xfail - known bug\nXPASS tests/a.py::test_xpass - fixed already\n== 5 passed, 4 skipped, 1 xfailed, 1 xpassed in 0.1s ==\n")).unwrap();
    assert_eq!(o, "pytest: 5 passed, 4 skipped, 1 xfailed, 1 xpassed in 0.1s\nskipped: 3 not ready, 1 needs gpu\nxfailed: tests/a.py::test_xfail\nxpassed: tests/a.py::test_xpass\n");
}

#[test]
fn many_parametrized_failures_of_one_shape_are_folded_but_listed_by_id() {
    let mut body = String::from("=================================== FAILURES ===================================\n");
    let mut sum = String::from("=========================== short test summary info ===========================\n");
    for i in 0..12 {
        body += &format!("________________________________ test_p[{i}] ________________________________\ntests\\t.py:5: in test_p\n    assert f(n) == 1\nE   assert {} == 1\n", i + 7);
        sum += &format!("FAILED tests/t.py::test_p[{i}] - assert {} == 1\n", i + 7);
    }
    let o = run(&body, &format!("{sum}== 12 failed in 0.1s ==\n"));
    assert_eq!(o.lines().count(), 3, "{o}");
    assert!(o.contains("tests/t.py:5 test_p[0]: assert 7 == 1") && o.contains("+11 more test_p[..] with the same message: 1, 2, 3, 4, 5, ... 11"), "{o}");
}

#[test]
fn captured_output_is_counted_not_listed() {
    let body = "=================================== FAILURES ===================================\n__________________________________ test_a ___________________________________\n>   assert 0\nE   assert 0\n\nt.py:3: AssertionError\n----------------------------- Captured stdout call -----------------------------\nE not an error line\nE neither\n";
    let o = pytest(&lines(&format!("{body}=== short test summary info ===\nFAILED t.py::test_a - assert 0\n== 1 failed in 0.1s ==\n"))).unwrap();
    assert_eq!(o, "pytest: 1 failed in 0.1s\nt.py:3 test_a: assert 0\n(captured output of 1 tests omitted)\n");
}

#[test]
fn hundreds_of_failures_are_capped_with_a_count() {
    let mut body = String::from("=== FAILURES ===\n");
    let mut sum = String::from("=== short test summary info ===\n");
    for i in 0..90 {
        body += &format!("______ test_u{i} ______\nt.py:{i}: in test_u{i}\nE   Boom{i}: x\n");
        sum += &format!("FAILED t.py::test_u{i} - Boom{i}: x\n");
    }
    let o = run(&body, &format!("{sum}== 90 failed in 1.0s ==\n"));
    assert_eq!(o.lines().count(), 1 + 40 + 1, "{o}");
    assert!(o.ends_with("... +50 more failures\n"), "{o}");
}
