use super::ts_tsc::tsc;
use super::util::{re, t::*};

/// Is `file:line` named in the view, as `file:1,2,10:5` (place list) or `file:2 TS...` (single error)?
fn kept(out: &str, file: &str, line: u32) -> bool {
    out.lines().flat_map(|l| l.split_whitespace()).any(|w| {
        w.strip_prefix(file).and_then(|r| r.strip_prefix(':')).is_some_and(|r| r.split(',').any(|it| it.split(':').next() == Some(&line.to_string())))
    })
}

/// Every `file(l,c): error TSn` of a plain run must be in the view with its code.
fn assert_all_kept(input: &str, out: &str) {
    let e = re(r"^(.+?)\((\d+),\d+\): (?:error|warning) (TS\d+):");
    let mut n = 0;
    for c in input.lines().filter_map(|l| e.captures(l)) {
        n += 1;
        assert!(kept(out, &c[1], c[2].parse().unwrap()), "{}:{} lost in\n{out}", &c[1], &c[2]);
        assert!(out.contains(&c[3]), "{} lost in\n{out}", &c[3]);
    }
    assert!(n > 0);
}

const MIXED: &str = "src/a.ts(4,3): error TS2741: Property 'email' is missing in type '{ id: number; name: string; }' but required in type 'User'.
src/a.ts(7,14): error TS2322: Type 'string' is not assignable to type 'number'.
src/a.ts(8,14): error TS2322: Type 'string' is not assignable to type 'number'.
src/a.ts(10,19): error TS7006: Parameter 'a' implicitly has an 'any' type.
src/a.ts(10,22): error TS7006: Parameter 'b' implicitly has an 'any' type.
src/a.ts(13,13): error TS2304: Cannot find name 'missingName'.
src/a.ts(14,13): error TS2304: Cannot find name 'otherMissing'.
src/a.ts(14,27): error TS2304: Cannot find name 'missingName'.
src/a.ts(25,25): error TS2307: Cannot find module './nope' or its corresponding type declarations.
src/b.ts(10,1): error TS2554: Expected 2 arguments, but got 1.
src/b.ts(11,11): error TS2554: Expected 2 arguments, but got 3.
src/b.ts(14,31): error TS2322: Type 'number' is not assignable to type 'string'.
src/b.ts(14,34): error TS2322: Type 'number' is not assignable to type 'string'.
src/d.ts(2,14): error TS2322: Type '(a: number) => string' is not assignable to type '(a: string) => string'.
  Types of parameters 'a' and 'a' are incompatible.
    Type 'string' is not assignable to type 'number'.
src/d.ts(3,14): error TS2322: Type '(a: number) => string' is not assignable to type '(a: string) => string'.
  Types of parameters 'a' and 'a' are incompatible.
    Type 'string' is not assignable to type 'number'.
";

#[test]
fn thirty_four_identical_errors_become_one_message_and_a_line_list() {
    let at = [1, 2, 4, 6, 7, 8, 10, 12, 13, 14, 16, 18, 19, 20, 22, 24, 25, 26, 28, 30, 31, 32, 34, 36, 37, 38, 40, 42, 43, 44, 46, 48, 49, 50];
    let s: String = at.iter().map(|l| format!("a.ts({l},{}): error TS2322: Type 'string' is not assignable to type 'number'.\n", 14 + l % 3)).collect();
    let l = lines(&s);
    let o = small(tsc(&l), &l);
    assert_eq!(o.lines().next().unwrap(), "TS2322 x34: Type 'string' is not assignable to type 'number'.");
    assert_eq!(o.lines().nth(1).unwrap(), format!("a.ts:{}", at.iter().map(|l| l.to_string()).collect::<Vec<_>>().join(",")));
    assert_eq!(o.lines().last().unwrap(), "Found 34 errors in 1 file.");
    assert!(o.len() * 4 < s.len(), "{} -> {}", s.len(), o.len());
}

#[test]
fn mixed_codes_in_several_files_keep_every_location() {
    let l = lines(MIXED);
    let o = small(tsc(&l), &l);
    assert_all_kept(MIXED, &o);
    assert!(o.contains("src/a.ts:7,8") && o.contains("src/b.ts:14:31,14:34"), "{o}");
    assert!(o.contains("TS2304 x2: Cannot find name 'missingName'.\nsrc/a.ts:13,14\n"), "{o}");
    assert!(o.contains("src/a.ts:14 TS2304 Cannot find name 'otherMissing'."), "{o}");
    assert!(o.contains("Found 15 errors in 3 files."), "{o}");
}

#[test]
fn elaboration_stays_under_the_group_message() {
    let l = lines(MIXED);
    let o = tsc(&l).unwrap();
    let head = o.lines().position(|x| x.starts_with("TS2322 x2: Type '(a: number) => string'")).expect(&o);
    assert_eq!(o.lines().nth(head + 1).unwrap(), "  Types of parameters 'a' and 'a' are incompatible.");
    assert_eq!(o.lines().nth(head + 2).unwrap(), "    Type 'string' is not assignable to type 'number'.");
    assert_eq!(o.lines().nth(head + 3).unwrap(), "src/d.ts:2,3");
    assert_eq!(o.matches("Types of parameters").count(), 1, "the elaboration is printed once per group");
}

#[test]
fn pretty_output_loses_the_code_frames_but_not_the_errors() {
    let s = "src/a.ts:4:3 - error TS2741: Property 'email' is missing in type '{ id: number; name: string; }' but required in type 'User'.

4   return { id: 1, name: \"x\" };
    ~~~~~~

  src/a.ts:1:44 - 'email' is declared here.
    1 interface User { id: number; name: string; email: string }
                                                ~~~~~

src/b.ts:7:14 - error TS2322: Type 'string' is not assignable to type 'number'.

7 export const n: number = \"x\";
               ~


Found 2 errors in 2 files.

Errors  Files
     1  src/a.ts:4
     1  src/b.ts:7
";
    let l = lines(s);
    let o = small(tsc(&l), &l);
    assert!(o.contains("src/a.ts:4 TS2741 Property 'email' is missing") && o.contains("src/b.ts:7 TS2322 Type 'string'"), "{o}");
    assert!(o.contains("  see src/a.ts:1: 'email' is declared here."), "{o}");
    assert!(!o.contains('~') && !o.contains("export const") && !o.contains("Errors  Files") && !o.contains("other line"), "{o}");
    assert!(o.trim_end().ends_with("Found 2 errors in 2 files."));
}

#[test]
fn watch_mode_banners_are_dropped_and_its_summary_kept() {
    let l = lines("[10:42:01] Starting compilation in watch mode...\n\nsrc/c.ts(4,2): error TS2551: Property 'nme' does not exist on type 'User'. Did you mean 'name'?\n\n[10:42:03] Found 1 error. Watching for file changes.\n");
    let o = tsc(&l).unwrap();
    assert!(o.starts_with("src/c.ts:4 TS2551 Property 'nme'") && o.ends_with("Found 1 error. Watching for file changes.\n"), "{o}");
    assert!(!o.contains("Starting compilation") && !o.contains("other line"));
    let clean = tsc(&lines("[10:44:00] Found 0 errors. Watching for file changes.")).unwrap();
    assert_eq!(clean, "Found 0 errors. Watching for file changes.\n");
}

#[test]
fn errors_without_a_location_are_kept_with_their_code() {
    let l = lines("error TS18003: No inputs were found in config file 'tsconfig.json'. Specified 'include' paths were '[\"**/*\"]'.\ntsconfig.json(3,5): error TS5023: Unknown compiler option 'foo'.\n");
    let o = tsc(&l).unwrap();
    assert!(o.contains("TS18003 No inputs were found") && o.contains("tsconfig.json:3 TS5023 Unknown compiler option 'foo'."), "{o}");
}

#[test]
fn lines_it_cannot_read_are_counted_and_a_short_count_hands_the_text_back() {
    let l = lines("a line no tsc prints\nsrc/a.ts(1,1): error TS1005: ';' expected.\nFound 1 error in src/a.ts:1\n");
    assert!(tsc(&l).unwrap().contains("(1 other line omitted)"));
    let wrapped = lines("> app@1.0.0 typecheck\n> tsc --noEmit\n\nsrc/a.ts(1,1): error TS1005: ';' expected.\nnpm error Lifecycle script `typecheck` failed\n");
    assert!(!tsc(&wrapped).unwrap().contains("omitted"), "runner chatter is not a lost diagnostic");
    let short = lines("src/a.ts(1,1): error TS1005: ';' expected.\nFound 3 errors in 2 files.\n");
    assert!(tsc(&short).is_none(), "the tool counted 3, only 1 could be read");
    assert!(tsc(&lines("Version 5.4.2")).is_none() && tsc(&lines("all fine")).is_none());
}

#[test]
fn vue_tsc_and_build_mode_use_the_same_view() {
    let l = lines("src/App.vue(12,7): error TS2322: Type 'number' is not assignable to type 'string'.\nsrc/B.vue(3,7): error TS2322: Type 'number' is not assignable to type 'string'.\nsrc/C.vue(9,1): error TS2322: Type 'number' is not assignable to type 'string'.\n");
    for cmd in ["vue-tsc --noEmit", "tsc -b", "npx tsc --build --force"] {
        let o = super::view(cmd, &l).unwrap_or_else(|| panic!("{cmd}"));
        assert!(o.contains("TS2322 x3: Type 'number'") && o.contains("src/App.vue:12 src/B.vue:3 src/C.vue:9"), "{cmd}: {o}");
    }
}
