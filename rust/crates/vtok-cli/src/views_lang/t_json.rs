use super::view;
use super::util::t::{lines, small};

fn run(cmd: &str, input: &str) -> String {
    let l = lines(input);
    small(view(cmd, &l), &l)
}

#[test]
fn playwright_json_failures() {
    let mut passed = String::new();
    for i in 0..6 { passed += &format!(r#"{{"title":"ok {i}","ok":true,"tests":[{{"status":"expected","projectName":"chromium","results":[{{"status":"passed","errors":[],"duration":10}}]}}]}},"#); }
    let s = format!(r#"{{"stats":{{"expected":6,"unexpected":1,"skipped":0,"flaky":0,"duration":2500.0}},
"suites":[{{"title":"a.spec.ts","file":"a.spec.ts","specs":[{passed}
{{"title":"fails","file":"a.spec.ts","line":9,"ok":false,"tests":[{{"status":"unexpected","projectName":"chromium","results":[{{"status":"failed","errors":[{{"message":"\u001b[31mError: expected Home\nReceived: Login\nat x"}}],"duration":5}}]}}]}}],"suites":[]}}],"errors":[]}}"#);
    let o = run("npx playwright test --reporter=json", &s);
    assert!(o.contains("6 passed, 1 failed") && o.contains("FAIL a.spec.ts:9 > fails [chromium]"), "{o}");
    assert!(o.contains("Received: Login") && !o.contains("ok 3") && !o.contains('\u{1b}'));
}

#[test]
fn rspec_json_failures() {
    let mut ex = String::new();
    for i in 0..8 { ex += &format!(r#"{{"id":"{i}","full_description":"S t{i}","status":"passed","file_path":"./spec/a_spec.rb","line_number":{i},"run_time":0.01,"pending_message":null,"exception":null}},"#); }
    let s = format!(r#"{{"version":"3.12.0","examples":[{ex}{{"id":"9","full_description":"User is valid","status":"failed","file_path":"./spec/user_spec.rb","line_number":7,"run_time":0.1,"exception":{{"class":"RSpec::Expectations::ExpectationNotMetError","message":"expected true\n got false","backtrace":["/x/gems/rspec/a.rb:1","/app/spec/user_spec.rb:8"]}}}}],"summary":{{"duration":0.5,"example_count":9,"failure_count":1,"pending_count":0,"errors_outside_of_examples_count":0}},"summary_line":"9 examples, 1 failure"}}"#);
    let o = run("bundle exec rspec --format json", &s);
    assert!(o.contains("9 examples, 1 failure") && o.contains("FAIL spec/user_spec.rb:7 User is valid"), "{o}");
    assert!(o.contains("expected true") && o.contains("at /app/spec/user_spec.rb:8") && !o.contains("S t3"));
}

#[test]
fn rubocop_json_offenses() {
    let off = r#"{"severity":"convention","message":"Trailing whitespace detected.","cop_name":"Layout/TrailingWhitespace","correctable":true,"location":{"start_line":10,"start_column":5,"last_line":10,"last_column":8,"length":3}}"#;
    let s = format!(r#"{{"metadata":{{"rubocop_version":"1.60.0","ruby_engine":"ruby"}},"files":[{{"path":"app/a.rb","offenses":[{off},{off}]}},{{"path":"app/b.rb","offenses":[]}}],"summary":{{"offense_count":2,"inspected_file_count":2}}}}"#);
    let o = run("rubocop --format json", &s);
    assert!(o.contains("app/a.rb:10:5 C Layout/TrailingWhitespace") && o.contains("2 offenses in 1 of 2 files (2 correctable)"), "{o}");
}

#[test]
fn eslint_json_trims_common_dir() {
    let m = r#"{"ruleId":"no-unused-vars","severity":2,"message":"'x' is defined but never used.","line":3,"column":7}"#;
    let s = format!(r#"[{{"filePath":"/home/me/proj/src/a.ts","messages":[{m}],"errorCount":1,"warningCount":0,"source":"const x = 1;"}},{{"filePath":"/home/me/proj/src/b/c.ts","messages":[],"errorCount":0,"warningCount":0}}]"#);
    let o = run("npx eslint -f json src", &s);
    assert!(o.contains("a.ts:3:7 E no-unused-vars") && !o.contains("/home/me") && o.contains("1 errors, 0 warnings in 2 files"), "{o}");
}

#[test]
fn sqlfluff_json_violations() {
    let s = r#"[{"filepath":"models/a.sql","violations":[{"line_no":1,"line_pos":1,"code":"LT09","description":"Select wildcard used.","fixes":[],"warning":false,"name":"layout.select_targets"}]},{"filepath":"models/b.sql","violations":[]}]"#;
    let o = run("sqlfluff lint models --format json", s);
    assert!(o.contains("models/a.sql:1:1 LT09 Select wildcard used.") && o.contains("1 violations in 1 files"), "{o}");
}

#[test]
fn phpstan_json_messages() {
    let s = r#"{"totals":{"errors":0,"file_errors":2},"files":{"app/User.php":{"errors":2,"messages":[{"message":"Property $id does not accept null.","line":10,"ignorable":true},{"message":"Call to undefined method Model::find().","line":25,"ignorable":false}]}},"errors":[]}"#;
    let o = run("vendor/bin/phpstan analyse --error-format=json", s);
    assert!(o.contains("app/User.php:10 Property $id") && o.contains("phpstan: 2 errors in 1 files"), "{o}");
}

#[test]
fn json_views_reject_other_output() {
    assert!(view("rspec", &lines("{\"a\": 1}")).is_none());
    assert!(view("playwright test", &lines("hello")).is_none());
}
