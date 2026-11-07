use super::{ls, run, view};

const MVN: &str = "[INFO] Scanning for projects...
[INFO] ------------------------< com.acme:app >-------------------------
[INFO] Building app 1.0-SNAPSHOT
[INFO] --------------------------------[ jar ]---------------------------------
Downloading from central: https://repo.maven.apache.org/maven2/junit/junit/4.13/junit-4.13.pom
Downloaded from central: https://repo.maven.apache.org/maven2/junit/junit/4.13/junit-4.13.pom (1.5 kB at 10 kB/s)
[INFO] --- maven-compiler-plugin:3.8.1:compile (default-compile) @ app ---
[WARNING] /src/A.java: uses unchecked or unsafe operations.
[WARNING] /src/B.java: uses unchecked or unsafe operations.
[INFO] --- maven-surefire-plugin:2.22.2:test (default-test) @ app ---
[INFO] Running com.acme.GoodTest
[INFO] Tests run: 4, Failures: 0, Errors: 0, Skipped: 0, Time elapsed: 0.05 s - in com.acme.GoodTest
[INFO] Running com.acme.BadTest
[ERROR] Tests run: 2, Failures: 1, Errors: 0, Skipped: 0, Time elapsed: 0.04 s <<< FAILURE! - in com.acme.BadTest
[ERROR] testSum  Time elapsed: 0.01 s  <<< FAILURE!
org.opentest4j.AssertionFailedError: expected: <4> but was: <5>
\tat org.junit.jupiter.api.AssertionUtils.fail(AssertionUtils.java:55)
\tat com.acme.BadTest.testSum(BadTest.java:12)
[INFO]
[INFO] Results:
[ERROR] Failures:
[ERROR]   BadTest.testSum:12 expected: <4> but was: <5>
[INFO]
[ERROR] Tests run: 6, Failures: 1, Errors: 0, Skipped: 0
[INFO] ------------------------------------------------------------------------
[INFO] BUILD FAILURE
[INFO] ------------------------------------------------------------------------
[INFO] Total time:  3.456 s
[INFO] Finished at: 2024-01-01T10:00:00Z
[ERROR] Failed to execute goal org.apache.maven.plugins:maven-surefire-plugin:2.22.2:test (default-test) on project app: There are test failures.
[ERROR]
[ERROR] -> [Help 1]
[ERROR] Re-run Maven using the -X switch to enable full debug logging.
";

#[test]
fn mvn_keeps_failures_and_result() {
    let o = run("./mvnw -q clean verify", MVN);
    assert!(o.contains("BadTest.testSum:12 expected: <4> but was: <5>"), "{o}");
    assert!(o.contains("AssertionFailedError: expected: <4>"));
    assert!(o.contains("Tests run: 6, Failures: 1") && o.contains("BUILD FAILURE") && o.contains("Total time:"));
    assert!(o.contains("Failed to execute goal"));
    assert!(o.contains("(2 warnings dropped)") && o.contains("(1 passing test classes)"));
    assert!(!o.contains("Downloading") && !o.contains("Scanning") && !o.contains("AssertionUtils") && !o.contains("Help 1"));
}

#[test]
fn mvn_compile_errors_and_reactor() {
    let s = "[INFO] Reactor Summary for parent 1.0:\n[INFO] core ................................................ SUCCESS [  1.2 s]\n[INFO] web ................................................ FAILURE [  0.3 s]\n[INFO] cli ................................................ SKIPPED\n[INFO] BUILD FAILURE\n[ERROR] COMPILATION ERROR :\n[ERROR] /src/web/W.java:[10,5] cannot find symbol\n[ERROR]   symbol:   variable x\n[INFO] 1 error\n";
    let o = run("mvn package", s);
    assert!(o.contains("web ....") && o.contains("cli ....") && !o.contains("core ...."));
    assert!(o.contains("W.java:[10,5] cannot find symbol") && o.contains("(1 modules SUCCESS)"));
}

#[test]
fn mvn_wrong_output_is_none() {
    assert!(view("mvn test", &ls("some random text")).is_none());
}
