use super::quote::templates;
use super::{render, D};

const NOUN: (&str, &str) = ("error", "errors");

fn d(file: &str, line: u32, col: u32, code: &str, msg: &str) -> D { D::new(file, line, col, code, msg) }

#[test]
fn identical_messages_share_one_header_and_a_line_list() {
    let ds: Vec<D> = [1, 2, 4, 9].iter().map(|&l| d("a.ts", l, 14, "TS2322", "Type 'string' is not assignable to type 'number'.")).collect();
    let o = render(&ds, NOUN, None);
    assert_eq!(o, vec!["TS2322 x4: Type 'string' is not assignable to type 'number'.", "a.ts:1,2,4,9", "Found 4 errors in 1 file."]);
}

#[test]
fn the_column_appears_only_where_two_hits_share_a_line() {
    let ds = vec![d("a.ts", 14, 13, "TS2304", "Cannot find name 'x'."), d("a.ts", 14, 27, "TS2304", "Cannot find name 'x'."), d("a.ts", 20, 3, "TS2304", "Cannot find name 'x'."), d("b.ts", 7, 1, "TS2304", "Cannot find name 'x'.")];
    assert_eq!(render(&ds, NOUN, None)[1], "a.ts:14:13,14:27,20 b.ts:7");
}

#[test]
fn a_single_error_is_one_line_and_elaboration_stays_under_it() {
    let mut one = d("src/d.ts", 2, 14, "TS2322", "Type '(a: number) => string' is not assignable to type '(a: string) => string'.");
    one.extra = vec!["Types of parameters 'a' and 'a' are incompatible.".into(), "  Type 'string' is not assignable to type 'number'.".into()];
    let o = render(&[one], NOUN, None);
    assert_eq!(o[0], "src/d.ts:2 TS2322 Type '(a: number) => string' is not assignable to type '(a: string) => string'.");
    assert_eq!(o[1], "  Types of parameters 'a' and 'a' are incompatible.");
    assert_eq!(o[2], "    Type 'string' is not assignable to type 'number'.");
}

#[test]
fn different_elaboration_means_different_groups() {
    let mut a = d("a.ts", 1, 1, "TS2322", "Type 'A' is not assignable to type 'B'.");
    let mut b = a.clone();
    a.extra = vec!["Property 'x' is missing.".into()];
    b.line = 2;
    b.extra = vec!["Property 'y' is missing.".into()];
    let o = render(&[a, b], NOUN, None).join("\n");
    assert!(o.contains("Property 'x' is missing.") && o.contains("Property 'y' is missing."), "{o}");
}

#[test]
fn singletons_that_differ_in_one_name_merge_into_a_template() {
    let ds: Vec<D> = (0..5).map(|i| d("b.js", 2 + 3 * i, 10, "no-unused-vars", &format!("'f{i}' is defined but never used."))).collect();
    let o = render(&ds, ("problem", "problems"), None);
    assert_eq!(o[0], "no-unused-vars x5: '…' is defined but never used.");
    assert_eq!(o[1], "b.js:2 f0,5 f1,8 f2,11 f3,14 f4");
    let two: Vec<D> = (0..2).map(|i| d("b.js", i + 1, 1, "no-undef", &format!("'v{i}' is not defined."))).collect();
    assert!(render(&two, NOUN, None)[0].starts_with("b.js:1 no-undef 'v0'"), "two are not worth a template");
}

#[test]
fn repeated_names_stay_exact_groups() {
    let mut ds = vec![];
    for i in 0..4 { ds.push(d("a.ts", i + 1, 1, "TS2304", "Cannot find name 'foo'.")); }
    for i in 0..3 { ds.push(d("a.ts", i + 10, 1, "TS2304", &format!("Cannot find name 'bar{i}'."))); }
    let o = render(&ds, NOUN, None);
    assert_eq!(o[0], "TS2304 x4: Cannot find name 'foo'.");
    assert_eq!(o[2], "TS2304 x3: Cannot find name '…'.");
}

#[test]
fn warnings_follow_errors_with_a_label() {
    let mut w = d("a.js", 3, 1, "no-console", "Unexpected console statement.");
    w.warn = true;
    let o = render(&[w, d("a.js", 9, 1, "no-var", "Unexpected var.")], ("problem", "problems"), None);
    assert_eq!(o[0], "a.js:9 no-var Unexpected var.");
    assert_eq!(o[1], "a.js:3 warning no-console Unexpected console statement.");
}

#[test]
fn kinds_past_the_cap_are_named_not_dropped() {
    let ds: Vec<D> = (0..45).map(|i| d("a.ts", i + 1, 1, &format!("TS{}", 1000 + i), &format!("distinct message number {i}"))).collect();
    let o = render(&ds, NOUN, None);
    assert_eq!(o.len(), 40 + 1 + 1);
    assert!(o[40].starts_with("+5 more kinds (5 errors): TS1040 a.ts:41, TS1041 a.ts:42") && o[40].ends_with("TS1044 a.ts:45"), "{}", o[40]);
}

#[test]
fn a_long_place_list_says_how_many_it_left_out() {
    let ds: Vec<D> = (1..=200).map(|i| d("a.ts", i, 1, "TS2322", "same")).collect();
    let o = render(&ds, NOUN, None);
    assert!(o.iter().any(|l| l == "... +50 more places"), "{o:?}");
}

#[test]
fn absolute_paths_below_the_base_become_relative() {
    use super::path::relative_to;
    assert_eq!(relative_to("C:\\Users\\me\\app\\src\\a.js", "c:/Users/me/app"), "src/a.js");
    assert_eq!(relative_to("/home/me/app/src/a.js", "/home/me/app"), "src/a.js");
    assert_eq!(relative_to("/home/me/app2/a.js", "/home/me/app"), "/home/me/app2/a.js", "a sibling with the same prefix is not below");
    assert_eq!(relative_to("src\\a.js", "/home/me/app"), "src/a.js");
    assert_eq!(relative_to("/elsewhere/a.js", ""), "/elsewhere/a.js");
}

#[test]
fn a_huge_message_is_cut_but_keeps_its_start() {
    let long = format!("Type '{}' is not assignable", "x".repeat(500));
    let o = render(&[d("a.ts", 1, 1, "TS2322", &long)], NOUN, None);
    assert!(o[0].len() < 340 && o[0].starts_with("a.ts:1 TS2322 Type 'xxx") && o[0].ends_with("..."), "{}", o[0]);
}

#[test]
fn rule_less_diagnostics_have_no_stray_spaces() {
    let o = render(&[d("a.js", 3, 1, "", "Parsing error: Unexpected token")], ("problem", "problems"), None);
    assert_eq!(o[0], "a.js:3 Parsing error: Unexpected token");
    let three: Vec<D> = (1..=3).map(|i| d("a.js", i, 1, "", "Parsing error: Unexpected token")).collect();
    assert_eq!(render(&three, ("problem", "problems"), None)[..2], ["x3: Parsing error: Unexpected token", "a.js:1,2,3"]);
    assert_eq!(render(&[d("", 0, 0, "TS18003", "No inputs were found")], NOUN, None), ["TS18003 No inputs were found", "Found 1 error."]);
}

#[test]
fn quoted_names_are_found_but_apostrophes_in_words_are_not() {
    assert_eq!(templates("Cannot find name 'foo'."), vec![("Cannot find name '…'.".to_string(), "foo".to_string())]);
    assert_eq!(templates("`os` imported but unused"), vec![("`…` imported but unused".to_string(), "os".to_string())]);
    let two = templates("Property 'a' does not exist on type 'B'.");
    assert_eq!(two.iter().map(|t| t.0.as_str()).collect::<Vec<_>>(), ["Property '…' does not exist on type 'B'.", "Property 'a' does not exist on type '…'."]);
    assert!(templates("Don't use var, it isn't scoped").is_empty());
    assert!(templates("no quotes at all").is_empty());
    assert!(templates("'a' 'b' 'c' 'd' are four names").is_empty(), "prose with many names is not a template");
}

#[test]
fn with_two_names_the_one_that_varies_is_masked() {
    let ds: Vec<D> = ["a", "b", "c"].iter().enumerate().map(|(i, n)| d("x.ts", i as u32 + 1, 5, "TS7006", &format!("Parameter '{n}' implicitly has an 'any' type."))).collect();
    assert_eq!(render(&ds, NOUN, None)[..2], ["TS7006 x3: Parameter '…' implicitly has an 'any' type.", "x.ts:1 a,2 b,3 c"]);
    let by_type: Vec<D> = (0..4).map(|i| d("y.ts", i + 1, 1, "TS2339", &format!("Property 'p{i}' does not exist on type 'Foo'."))).collect();
    assert_eq!(render(&by_type, NOUN, None)[0], "TS2339 x4: Property '…' does not exist on type 'Foo'.");
    let mixed = vec![d("y.ts", 1, 1, "TS2339", "Property 'p' does not exist on type 'A'."), d("y.ts", 2, 1, "TS2339", "Property 'q' does not exist on type 'B'."), d("y.ts", 3, 1, "TS2339", "Property 'r' does not exist on type 'C'.")];
    assert_eq!(render(&mixed, NOUN, None).len(), 4, "three different types: nothing in common but the code, three lines");
}
