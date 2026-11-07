use super::{ls, run, view};

#[test]
fn golangci_json_groups_by_linter_and_file() {
    let iss = |l: &str, t: &str, f: &str, line: u32| format!(r#"{{"FromLinter":"{l}","Text":"{t}","Pos":{{"Filename":"{f}","Line":{line},"Column":5,"Offset":10}},"SourceLines":["x := f()"],"Severity":""}}"#);
    let s = format!(r#"{{"Issues":[{},{},{}],"Report":{{"Linters":[{{"Name":"errcheck","Enabled":true}}]}}}}"#,
        iss("errcheck", "Error return value not checked", "main.go", 42), iss("errcheck", "Error return value not checked", "main.go", 50), iss("gosimple", "Should use strings.Contains", "utils.go", 15));
    let o = run("golangci-lint run --out-format json", &s);
    assert!(o.contains("3 issues: errcheck 2, gosimple 1") && o.contains("main.go 42:5,50:5 Error return value not checked") && o.contains("utils.go 15:5"), "{o}");
    assert!(!o.contains("SourceLines"));
    let none = r#"{"Issues":null,"Report":{"Linters":[{"Name":"a"},{"Name":"b"},{"Name":"c"},{"Name":"d"}]}}"#;
    assert!(run("golangci-lint run --output.json.path stdout", none).contains("0 issues"));
}

#[test]
fn trx_failed_tests() {
    let s = "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<TestRun xmlns=\"http://microsoft.com/schemas/VisualStudio/TeamTest/2010\">\n  <Results>\n    <UnitTestResult executionId=\"1\" testName=\"Ns.A.Works\" duration=\"00:00:00.01\" outcome=\"Passed\" />\n    <UnitTestResult executionId=\"2\" testName=\"Ns.A.Breaks\" duration=\"00:00:00.02\" outcome=\"Failed\">\n      <Output><ErrorInfo><Message>Assert.Equal() Failure: 1 != 2</Message><StackTrace>   at Ns.A.Breaks() in /src/A.cs:line 12\n   at System.Foo()</StackTrace></ErrorInfo></Output>\n    </UnitTestResult>\n  </Results>\n  <ResultSummary outcome=\"Failed\"><Counters total=\"2\" executed=\"2\" passed=\"1\" failed=\"1\" error=\"0\" /></ResultSummary>\n</TestRun>";
    let o = run("dotnet test --logger trx", s);
    assert!(o.contains("trx Failed: 2 total, 1 passed, 1 failed") && o.contains("FAIL Ns.A.Breaks") && o.contains("1 != 2") && o.contains("A.cs:line 12"), "{o}");
    assert!(!o.contains("Works") && !o.contains("System.Foo"));
}

#[test]
fn go_test_json_keeps_build_errors() {
    let s = "# ex/p [ex/p.test]\n./a_test.go:5:2: undefined: missingFunc\n{\"Action\":\"start\",\"Package\":\"ex/p\"}\n{\"Action\":\"output\",\"Package\":\"ex/p\",\"Output\":\"FAIL\\tex/p [build failed]\\n\"}\n{\"Action\":\"fail\",\"Package\":\"ex/p\",\"Elapsed\":0}\n{\"Action\":\"start\",\"Package\":\"ex/q\"}\n{\"Action\":\"output\",\"Package\":\"ex/q\",\"Output\":\"ok  \\tex/q\\t0.01s\\n\"}\n{\"Action\":\"pass\",\"Package\":\"ex/q\",\"Elapsed\":0.01}\n";
    let o = run("go test -json ./...", s);
    assert!(o.contains("./a_test.go:5:2: undefined: missingFunc") && o.contains("1 failed"), "{o}");
    assert!(view("go test -json ./...", &ls("")).is_none());
}
