//! phpunit, pest, paratest and `artisan test`: failing tests with message + file:line, summary line.
use super::util::*;

pub fn tests(lines: &[String]) -> Option<String> {
    let phpunit_style = has(lines, r"^There (was|were) \d+ ") || has(lines, r"^(OK|OK, but|FAILURES!|ERRORS!|WARNINGS!|No tests executed)");
    let pest_style = has(lines, r"^\s*Tests:\s+.*\(\d+ assertions\)|^\s*(PASS|FAIL)\s+\S");
    if !phpunit_style && !pest_style { return None; }
    let mut out = vec![];
    if phpunit_style {
        out.extend(grep(lines, r"^There (was|were) \d+ "));
        let numbered = re(r"^\d+\) ");
        let stop = |l: &str| re(r"^(FAILURES!|ERRORS!|WARNINGS!|OK|Tests: |There (was|were))").is_match(l);
        out.extend(blocks(lines, &|l| numbered.is_match(l), &stop, 10, &|l| l.contains("/vendor/")));
        out.extend(grep(lines, r"^(Time: |OK \(|OK, but|FAILURES!|ERRORS!|WARNINGS!|Tests: |No tests executed)"));
    } else {
        out.extend(collapse(grep(lines, r"^\s*FAIL\s+\S")));
        let start = |l: &str| re(r"^\s*(•|FAILED|WARN|ERROR)\s+\S").is_match(l);
        let stop = |l: &str| re(r"^\s*(Tests:|Duration:)").is_match(l);
        let drop = |l: &str| l.contains('▕') || l.contains("/vendor/") || l.trim() == "---" || re(r"^\s*\d+\s+\|").is_match(l);
        out.extend(blocks(lines, &start, &stop, 10, &drop));
        out.extend(grep(lines, r"^\s*(Tests:|Duration:)"));
    }
    finish(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::util::t::*;

    #[test]
    fn phpunit_failure() {
        let s = "PHPUnit 10.5.0 by Sebastian Bergmann and contributors.\n\nRuntime:       PHP 8.2.1\nConfiguration: /app/phpunit.xml\n\n.......F.......................                  30 / 30 (100%)\n\nTime: 00:00.120, Memory: 14.00 MB\n\nThere was 1 failure:\n\n1) Tests\\UserTest::testName\nFailed asserting that two strings are equal.\n--- Expected\n+++ Actual\n@@ @@\n-'a'\n+'b'\n\n/app/tests/UserTest.php:15\n/app/vendor/phpunit/phpunit/src/Framework/TestCase.php:1\n\nFAILURES!\nTests: 30, Assertions: 31, Failures: 1.\n";
        let l = lines(s);
        let o = small(tests(&l), &l);
        assert!(o.contains("Tests\\UserTest::testName") && o.contains("Failed asserting") && o.contains("/app/tests/UserTest.php:15"));
        assert!(o.contains("Tests: 30, Assertions: 31, Failures: 1.") && o.contains("There was 1 failure"));
        assert!(!o.contains("Sebastian") && !o.contains("/vendor/") && !o.contains("Runtime:"));
        let ok = lines("PHPUnit 10.5.0 by Sebastian Bergmann and contributors.\n\n....  4 / 4 (100%)\n\nTime: 00:00.01, Memory: 6.00 MB\n\nOK (4 tests, 4 assertions)\n");
        let o = tests(&ok).unwrap();
        assert!(o.contains("OK (4 tests, 4 assertions)") && !o.contains("PHPUnit 10"));
    }

    #[test]
    fn pest_and_artisan_failure() {
        let mut s = String::new();
        for i in 0..6 { s += &format!("   PASS  Tests\\Unit\\Ok{i}Test\n  ✓ it works {i}                    0.01s\n"); }
        s += "   FAIL  Tests\\Feature\\UserTest\n  ⨯ it creates a user                    0.02s\n  ---\n\n  • Tests\\Feature\\UserTest > it creates a user\n  Expected response status code [201] but received 500.\n  Failed asserting that 201 is identical to 500.\n\n  at tests/Feature/UserTest.php:21\n     17▕     $r = $this->post('/users');\n     21▕     $r->assertStatus(201);\n  1▕ x\n\n  Tests:    1 failed, 6 passed (8 assertions)\n  Duration: 0.34s\n";
        let l = lines(&s);
        let o = small(tests(&l), &l);
        assert!(o.contains("FAIL  Tests\\Feature\\UserTest") && o.contains("received 500") && o.contains("at tests/Feature/UserTest.php:21"));
        assert!(o.contains("Tests:    1 failed, 6 passed") && o.contains("Duration: 0.34s"));
        assert!(!o.contains("Ok3Test") && !o.contains("assertStatus"));
    }
}
