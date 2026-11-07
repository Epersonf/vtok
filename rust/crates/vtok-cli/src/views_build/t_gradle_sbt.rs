use super::run;

const GRADLE: &str = "Starting a Gradle Daemon (subsequent builds will be faster)
> Configure project :app
> Task :app:compileJava
> Task :app:processResources UP-TO-DATE
> Task :app:classes
> Task :app:compileTestJava
src/test/java/FooTest.java:9: error: cannot find symbol
        int x = missing;
                ^
  symbol:   variable missing
> Task :app:test FAILED

FooTest > testBar() FAILED
    org.opentest4j.AssertionFailedError: expected: <1> but was: <2>
        at org.junit.jupiter.api.AssertionUtils.fail(AssertionUtils.java:55)
        at FooTest.testBar(FooTest.java:14)
        at java.base/java.lang.reflect.Method.invoke(Method.java:566)

5 tests completed, 1 failed

FAILURE: Build failed with an exception.

* What went wrong:
Execution failed for task ':app:test'.
> There were failing tests. See the report at: file:///build/reports/tests/test/index.html

* Try:
> Run with --stacktrace option to get the stack trace.
> Run with --info or --debug option to get more log output.

* Get more help at https://help.gradle.org

Deprecated Gradle features were used in this build, making it incompatible with Gradle 9.0.
BUILD FAILED in 8s
4 actionable tasks: 3 executed, 1 up-to-date
";

#[test]
fn gradle_failures() {
    let o = run("./gradlew test --console=plain", GRADLE);
    assert!(o.contains("FooTest.java:9: error: cannot find symbol"), "{o}");
    assert!(o.contains("FAILED FooTest > testBar()") && o.contains("expected: <1> but was: <2>") && o.contains("FooTest.java:14"));
    assert!(o.contains("> Task :app:test FAILED") && o.contains("BUILD FAILED in 8s") && o.contains("There were failing tests"));
    assert!(o.contains("5 tests completed, 1 failed"));
    assert!(!o.contains("Run with --stacktrace") && !o.contains("processResources") && !o.contains("Method.invoke") && !o.contains("int x = missing"));
}

const SBT: &str = "[info] welcome to sbt 1.9.7
[info] loading project definition from /p/project
[info] compiling 3 Scala sources to /p/target/classes ...
[warn] there was one deprecation warning; re-run with -deprecation for details
[warn] one warning found
[info] FooSpec:
[info] Foo
[info] - should add numbers
[info] - should subtract *** FAILED ***
[info]   3 did not equal 4 (FooSpec.scala:12)
[info]   at org.scalatest.Assertions.newAssertionFailedException(Assertions.scala:472)
[info] Run completed in 400 milliseconds.
[info] Total number of tests run: 2
[info] Suites: completed 1, aborted 0
[info] Tests: succeeded 1, failed 1, canceled 0, ignored 0, pending 0
[info] *** 1 TEST FAILED ***
[error] Failed tests:
[error] \tFooSpec
[error] (Test / test) sbt.TestsFailedException: Tests unsuccessful
[error] Total time: 5 s, completed Jan 1, 2024
";

#[test]
fn sbt_failures() {
    let o = run("sbt test", SBT);
    assert!(o.contains("should subtract *** FAILED ***") && o.contains("3 did not equal 4 (FooSpec.scala:12)"), "{o}");
    assert!(o.contains("Tests: succeeded 1, failed 1") && o.contains("[error] (Test / test)") && o.contains("(2 warnings dropped)"));
    assert!(!o.contains("loading project") && !o.contains("should add numbers") && !o.contains("Assertions.scala"));
}
