use super::run;

const TEXT: &str = "=== RUN   TestA
--- PASS: TestA (0.00s)
=== RUN   TestB
    b_test.go:14: got 3, want 4
--- FAIL: TestB (0.00s)
    --- FAIL: TestB/sub (0.00s)
        b_test.go:20: sub failed
FAIL
FAIL\texample.com/m/pkg\t0.004s
ok  \texample.com/m/other\t0.003s
ok  \texample.com/m/cached\t(cached)
?   \texample.com/m/cmd\t[no test files]
";

#[test]
fn go_test_text() {
    let o = run("go test ./...", TEXT);
    assert!(o.contains("--- FAIL: TestB") && o.contains("b_test.go:14: got 3, want 4") && o.contains("TestB/sub"));
    assert!(o.contains("FAIL\texample.com/m/pkg"));
    assert!(o.contains("go test: 2 ok, 1 FAIL, 1 no test files"), "{o}");
    assert!(!o.contains("=== RUN") && !o.contains("--- PASS"));
}

const JSON: &str = r#"{"Action":"start","Package":"ex/p"}
{"Action":"run","Package":"ex/p","Test":"TestA"}
{"Action":"output","Package":"ex/p","Test":"TestA","Output":"=== RUN   TestA\n"}
{"Action":"output","Package":"ex/p","Test":"TestA","Output":"--- PASS: TestA (0.00s)\n"}
{"Action":"pass","Package":"ex/p","Test":"TestA","Elapsed":0}
{"Action":"run","Package":"ex/p","Test":"TestB"}
{"Action":"output","Package":"ex/p","Test":"TestB","Output":"=== RUN   TestB\n"}
{"Action":"output","Package":"ex/p","Test":"TestB","Output":"    b_test.go:14: got 3, want 4\n"}
{"Action":"output","Package":"ex/p","Test":"TestB","Output":"--- FAIL: TestB (0.00s)\n"}
{"Action":"fail","Package":"ex/p","Test":"TestB","Elapsed":0}
{"Action":"output","Package":"ex/p","Output":"FAIL\n"}
{"Action":"fail","Package":"ex/p","Elapsed":0.01}
{"Action":"output","Package":"ex/q","Output":"ok  \tex/q\t0.01s\n"}
{"Action":"pass","Package":"ex/q","Elapsed":0.01}
"#;

#[test]
fn go_test_json() {
    let o = run("go test -json ./...", JSON);
    assert!(o.contains("FAIL ex/p.TestB") && o.contains("b_test.go:14: got 3, want 4"), "{o}");
    assert!(o.contains("packages 1 ok, 1 failed, 0 no tests; tests 1 passed, 1 failed, 0 skipped"));
    assert!(!o.contains("TestA") && !o.contains("=== RUN") && !o.contains("\"Action\""));
}

#[test]
fn go_build_errors_and_vet() {
    let s = "go: downloading github.com/a/b v1.0.0\ngo: downloading github.com/c/d v1.2.0\n# example.com/m/pkg\npkg/a.go:12:5: undefined: foo\npkg/a.go:12:5: undefined: foo\npkg/b.go:3:2: \"os\" imported and not used\n";
    let o = run("go build ./...", s);
    assert!(o.contains("# example.com/m/pkg") && o.contains("pkg/b.go:3:2"));
    assert_eq!(o.matches("undefined: foo").count(), 1);
    assert!(!o.contains("downloading"));
    let o = run("go vet ./...", "go: downloading a v1\ngo: downloading b v2\ngo: downloading c v3\n");
    assert!(o.contains("go vet: ok (3 modules downloaded)"));
}

#[test]
fn golangci_lint() {
    let s = "cmd/main.go:10:2: Error return value of `f.Close` is not checked (errcheck)\n\tdefer f.Close()\n\t      ^\ncmd/main.go:22:5: ineffectual assignment to x (ineffassign)\n\tx := 1\n\t^\ninternal/a.go:7:1: exported func A should have comment (revive)\nfunc A() {}\n^\n";
    let o = run("golangci-lint run ./...", s);
    assert!(o.contains("3 issues: errcheck 1, ineffassign 1, revive 1"), "{o}");
    assert!(o.contains("cmd/main.go:10:2: Error return value") && !o.contains("defer f.Close()"));
}
