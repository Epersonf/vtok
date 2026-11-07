use super::js_eslint::{json, text};
use super::util::t::*;

/// `blocks` repetitions of five rules, like a file full of old-style code (the shape of the benchmark fixture).
fn five_rules(blocks: usize) -> String {
    let mut s = String::from("/app/b.js\n");
    for i in 0..blocks {
        let r = 3 * i + 1;
        s += &format!("  {r}:1   error  Unexpected var, use let or const instead  no-var\n  {}:10  error  'f{i}' is defined but never used            no-unused-vars\n", r + 1);
        s += &format!("  {}:23  error  'b{i}' is not defined                       no-undef\n  {}:8   error  Expected '===' and instead saw '=='       eqeqeq\n  {}:14  error  Empty block statement                     no-empty\n", r + 1, r + 2, r + 2);
    }
    s + &format!("\n✖ {n} problems ({n} errors, 0 warnings)\n", n = blocks * 5)
}

#[test]
fn every_rule_survives_with_its_places_and_the_names_that_differ() {
    let s = five_rules(12);
    let l = lines(&s);
    let o = small(text(&l, 1), &l);
    assert_eq!(o.lines().next().unwrap(), "no-var x12: Unexpected var, use let or const instead");
    assert_eq!(o.lines().nth(1).unwrap(), "/app/b.js:1,4,7,10,13,16,19,22,25,28,31,34");
    for rule in ["no-var", "no-unused-vars", "no-undef", "eqeqeq", "no-empty"] { assert!(o.contains(&format!("{rule} x12")), "{rule} lost\n{o}"); }
    assert!(o.contains("no-unused-vars x12: '…' is defined but never used\n/app/b.js:2 f0,5 f1,8 f2,"), "{o}");
    assert!(o.contains("35 f11") && o.contains("35 b11"), "{o}");
    assert!(o.contains("eqeqeq x12: Expected '===' and instead saw '=='\n/app/b.js:3,6,9,"), "{o}");
    assert!(o.ends_with("✖ 60 problems (60 errors, 0 warnings)\n"), "{o}");
    assert!(o.len() * 4 < s.len(), "{} -> {}", s.len(), o.len());
}

#[test]
fn warnings_keep_rule_and_places_after_the_errors() {
    let mut s = String::from("/app/src/a.js\n  10:5  error  'x' is defined but never used  no-unused-vars\n");
    for i in 0..20 { s += &format!("  {}:1  warning  Unexpected console statement  no-console\n", 20 + i); }
    s += "\n/app/src/b.js\n  3:9  warning  Missing semicolon  semi\n\n✖ 22 problems (1 error, 21 warnings)\n  0 errors and 1 warning potentially fixable with the `--fix` option.\n";
    let l = lines(&s);
    let o = small(text(&l, 1), &l);
    assert!(o.starts_with("/app/src/a.js:10 no-unused-vars 'x' is defined but never used\n"), "{o}");
    assert!(o.contains("warning no-console x20: Unexpected console statement\n/app/src/a.js:20,21,22,23,24,25,26,27,28,29,30,31\n... +8 more places\n"), "{o}");
    assert!(o.contains("/app/src/b.js:3 warning semi Missing semicolon\n"), "{o}");
    assert!(o.ends_with("✖ 22 problems (1 error, 21 warnings)\n0 errors and 1 warning potentially fixable with the `--fix` option.\n"), "{o}");
}

#[test]
fn parse_errors_have_no_rule_and_are_still_listed() {
    let l = lines("/app/src/c.js\n  1:15  error  Parsing error: Unexpected token )\n\n/app/src/d.js\n  4:1  error  Parsing error: Unexpected token )\n  9:3  error  Parsing error: Unexpected token )\n\n✖ 3 problems (3 errors, 0 warnings)\n");
    let o = text(&l, 1).unwrap();
    assert!(o.starts_with("x3: Parsing error: Unexpected token )\n/app/src/c.js:1 /app/src/d.js:4,9\n"), "{o}");
}

#[test]
fn compact_and_unix_formats() {
    let compact = lines("/app/a.js: line 3, col 5, Error - 'x' is defined but never used. (no-unused-vars)\n/app/a.js: line 9, col 1, Warning - Unexpected console statement. (no-console)\n\n2 problems\n");
    let unix = lines("/app/a.js:3:5: 'x' is defined but never used. [Error/no-unused-vars]\n/app/a.js:9:1: Unexpected console statement. [Warning/no-console]\n\n2 problems\n");
    for l in [compact, unix] {
        let o = text(&l, 1).unwrap();
        assert!(o.contains("/app/a.js:3 no-unused-vars 'x' is defined but never used.\n") && o.contains("/app/a.js:9 warning no-console Unexpected console statement.\n"), "{o}");
        assert!(o.ends_with("2 problems\n"), "{o}");
    }
}

#[test]
fn unreadable_or_short_output_is_left_to_the_generic_path() {
    let short = lines("/app/a.js\n  1:1  error  Unexpected var  no-var\n  2:1  error  Unexpected var  no-var\n\n✖ 3 problems (3 errors, 0 warnings)\n");
    assert!(text(&short, 1).is_none(), "the tool counted 3, only 2 could be read");
    let few = lines("/app/a.js\n  1:1  error  Unexpected var  no-var\n");
    assert!(text(&few, 6).is_none() && text(&few, 1).is_some());
    assert!(text(&lines("Oops! Something went wrong! :(\n\nESLint: 9.1.0\n"), 1).is_none() && text(&[], 1).is_none());
}

#[test]
fn rules_past_the_group_cap_are_still_named_with_a_place() {
    let mut s = String::from("/app/a.js\n");
    for i in 0..60 { s += &format!("  {}:1  error  Problem number {i}  rule-{i}\n", i + 1); }
    let o = text(&lines(&s), 1).unwrap();
    for i in 0..60 { assert!(o.contains(&format!("rule-{i}")), "rule-{i} lost\n{o}"); }
    assert!(o.contains("+20 more kinds (20 problems): rule-40 /app/a.js:41,"), "{o}");
}

#[test]
fn json_report_groups_by_rule_and_counts_files() {
    let s = r#"[{"filePath":"/app/a.js","messages":[{"ruleId":"no-var","severity":2,"message":"Unexpected var, use let or const instead.","line":1,"column":1},{"ruleId":"no-var","severity":2,"message":"Unexpected var, use let or const instead.","line":4,"column":1},{"ruleId":"no-console","severity":1,"message":"Unexpected console statement.","line":6,"column":3},{"ruleId":null,"fatal":true,"severity":2,"message":"Parsing error: Unexpected token","line":9,"column":2}],"errorCount":3,"warningCount":1},{"filePath":"/app/b.js","messages":[],"errorCount":0,"warningCount":0}]"#;
    let l = lines(s);
    let o = json(&l).unwrap();
    assert_eq!(o, "no-var x2: Unexpected var, use let or const instead.\n/app/a.js:1,4\n/app/a.js:9 Parsing error: Unexpected token\n/app/a.js:6 warning no-console Unexpected console statement.\neslint: 3 errors, 1 warnings in 2 files\n");
    assert!(json(&lines("not json")).is_none() && json(&lines("[1,2]")).is_none());
    assert_eq!(json(&lines(r#"[{"filePath":"/app/a.js","messages":[]}]"#)).unwrap(), "eslint: 0 errors, 0 warnings in 1 files\n");
}

#[test]
fn the_dedicated_view_takes_json_before_text() {
    let l = lines(&five_rules(2));
    assert!(super::view("npx eslint src", &l).unwrap().contains("no-var x2"));
    assert!(super::view("pnpm exec eslint -f json .", &lines(r#"[{"filePath":"/a.js","messages":[{"ruleId":"semi","severity":1,"message":"Missing semicolon.","line":2,"column":9}]}]"#)).unwrap().contains("a.js:2 warning semi"));
}
