use super::js_test::vitest;
use super::util::t::*;

/// `vitest run` on the benchmark fixture (30 tests, two of them fail), colour off.
const FIXTURE: &str = r#" RUN  v5.1.0 /fx/ts

 ❯ a.test.ts (30 tests | 2 failed) 10ms
   × case 4 5ms
   × case 11 1ms

⎯⎯⎯⎯⎯⎯⎯ Failed Tests 2 ⎯⎯⎯⎯⎯⎯⎯

 FAIL  a.test.ts > case 4
AssertionError: expected 5 to be +0 // Object.is equality

- Expected
+ Received

- 0
+ 5

 ❯ a.test.ts:6:34
      4| test("case 2", ()=>{ expect(2+1).toBe(3); });
      5| test("case 3", ()=>{ expect(3+1).toBe(4); });
      6| test("case 4", ()=>{ expect(4+1).toBe(0); });
       |                                  ^
      7| test("case 5", ()=>{ expect(5+1).toBe(6); });

⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯[1/2]⎯

 FAIL  a.test.ts > case 11
AssertionError: expected 12 to be +0 // Object.is equality

- Expected
+ Received

- 0
+ 12

 ❯ a.test.ts:13:36
     11| test("case 9", ()=>{ expect(9+1).toBe(10); });
     13| test("case 11", ()=>{ expect(11+1).toBe(0); });
       |                                    ^
     14| test("case 12", ()=>{ expect(12+1).toBe(13); });

⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯[2/2]⎯


 Test Files  1 failed (1)
      Tests  2 failed | 28 passed (30)
   Start at  11:49:40
   Duration  190ms (import 39%, tests 28%, transform 20%, worker 13%)
"#;

/// The view of one failing test of `m.test.ts` whose output (after the header) is `block`.
fn run(block: &str) -> String {
    let s = format!("⎯⎯⎯⎯ Failed Tests 1 ⎯⎯⎯⎯\n\n FAIL  m.test.ts > math > obj\n{block}\n\n⎯⎯⎯⎯⎯⎯[1/1]⎯\n\n Test Files  1 failed (1)\n      Tests  1 failed (1)\n");
    vitest(&lines(&s)).expect("vitest view")
}

#[test]
fn benchmark_fixture_is_names_values_and_locations_only() {
    let l = lines(FIXTURE);
    let o = small(vitest(&l), &l);
    assert_eq!(o, "FAIL a.test.ts > case 4\nexpected 5 to be +0\n❯ a.test.ts:6:34\nFAIL a.test.ts > case 11\nexpected 12 to be +0\n❯ a.test.ts:13:36\nTest Files  1 failed (1)\nTests  2 failed | 28 passed (30)\nDuration  190ms\n");
}

#[test]
fn passing_files_and_library_frames_stay_out() {
    let mut s = String::new();
    for i in 0..8 { s += &format!(" ✓ src/ok{i}.test.ts (12 tests) {i}ms\n"); }
    s += " ❯ src/b.test.ts (2 tests | 1 failed) 8ms\n   × b > fails 3ms\n\n⎯⎯⎯⎯⎯⎯ Failed Tests 1 ⎯⎯⎯⎯⎯⎯\n\n FAIL  src/b.test.ts > b > fails\nAssertionError: expected 1 to be 2 // Object.is equality\n\n- Expected\n+ Received\n\n- 2\n+ 1\n\n ❯ src/b.test.ts:10:5\n      8| it('fails', () => {\n     10|   expect(a).toBe(2)\n ❯ node_modules/x/y.js:1:1\n\n⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯\n\n Test Files  1 failed | 8 passed (9)\n      Tests  1 failed | 90 passed (91)\n   Duration  1.2s\n";
    let l = lines(&s);
    let o = small(vitest(&l), &l);
    assert!(o.contains("FAIL src/b.test.ts > b > fails") && o.contains("expected 1 to be 2") && o.contains("❯ src/b.test.ts:10:5"));
    assert!(o.contains("Test Files  1 failed") && o.contains("Tests  1 failed") && o.contains("Duration  1.2s"));
    assert!(!o.contains("ok3.test.ts") && !o.contains("node_modules"));
    assert!(vitest(&lines("hello")).is_none());
}

#[test]
fn a_diff_with_context_stays_because_it_shows_more_than_the_message() {
    let o = run("AssertionError: expected { a: 1, b: 2 } to deeply equal { a: 1, b: 3 }\n\n- Expected\n+ Received\n\n@@ -1,4 +1,4 @@\n  {\n    \"a\": 1,\n-   \"b\": 3,\n+   \"b\": 2,\n  }\n\n ❯ m.test.ts:4:60\n      4|   test(\"adds objects\", () => {\n       |                                ^\n");
    assert!(o.contains("deeply equal { a: 1, b: 3 }\n- Expected\n+ Received\n@@ -1,4 +1,4 @@\n  {\n    \"a\": 1,\n-   \"b\": 3,\n+   \"b\": 2,\n  }\n❯ m.test.ts:4:60\n"), "{o}");
    assert!(!o.contains('^') && !o.contains("AssertionError"), "{o}");
}

#[test]
fn a_string_pair_goes_only_when_the_message_states_both_strings() {
    let said = run("AssertionError: expected 'hello world' to be 'hello there' // Object.is equality\n\nExpected: \"hello there\"\nReceived: \"hello world\"\n\n ❯ m.test.ts:5:49");
    assert!(said.contains("expected 'hello world' to be 'hello there'\n❯ m.test.ts:5:49") && !said.contains("Expected:") && !said.contains("Object.is"), "{said}");
    let unsaid = run("AssertionError: expected [Function] to not throw an error but 'Error: boom' was thrown\n\n- Expected:\nundefined\n\n+ Received:\n\"Error: boom\"\n\n ❯ m.test.ts:7:73");
    assert!(unsaid.contains("- Expected:\nundefined\n+ Received:\n\"Error: boom\"") && unsaid.contains("not throw an error"), "{unsaid}");
}

#[test]
fn a_value_inside_a_longer_number_is_not_stated() {
    let o = run("AssertionError: expected 12 to be 30 // Object.is equality\n\n- Expected\n+ Received\n\n- 3\n+ 12\n\n ❯ m.test.ts:1:1");
    assert!(o.contains("- Expected\n+ Received\n- 3\n+ 12\n"), "{o}");
    let both = run("AssertionError: expected 12 to be 3 // Object.is equality\n\n- Expected\n+ Received\n\n- 3\n+ 12\n\n ❯ m.test.ts:1:1");
    assert!(both.contains("expected 12 to be 3\n❯ m.test.ts:1:1") && !both.contains("Expected"), "{both}");
}

#[test]
fn other_errors_keep_their_class_and_text() {
    let o = run("TypeError: Cannot read properties of undefined (reading 'x')\n ❯ m.test.ts:8:58\n");
    assert!(o.starts_with("FAIL m.test.ts > math > obj\nTypeError: Cannot read properties of undefined (reading 'x')\n❯ m.test.ts:8:58\n"), "{o}");
    assert!(run("AssertionError: custom failure\n ❯ m.test.ts:2:1").contains("AssertionError: custom failure"));
    let t = run("Error: Test timed out in 20ms.\nIf this is a long-running test, pass a timeout value.\n ❯ m.test.ts:9:3");
    assert!(t.contains("Error: Test timed out in 20ms.\nIf this is a long-running test, pass a timeout value.\n❯ m.test.ts:9:3"), "{t}");
}

#[test]
fn a_green_run_is_the_counts_without_the_timing_breakdown() {
    let l = lines(" ✓ a.test.ts (3 tests) 4ms\n ✓ b.test.ts (2 tests) 3ms\n\n Snapshots  2 passed\n Test Files  2 passed (2)\n      Tests  5 passed (5)\n   Start at  11:49:40\n   Duration  190ms (import 39%, tests 28%, transform 20%, worker 13%)\n");
    assert_eq!(vitest(&l).unwrap(), "Snapshots  2 passed\nTest Files  2 passed (2)\nTests  5 passed (5)\nDuration  190ms\n");
}
