//! Retention audit: every dedicated diagnostics view gets 70 distinct rules/codes (more than any row cap), the later ones
//! rare, and must still name each of them. A class may be grouped or counted, never dropped.
type Row = fn(usize, usize) -> String;

/// (name, command line, one diagnostic of class `c` at line `n`, closing line, the text that names class `c`)
const CASES: &[(&str, &str, Row, &str, Row)] = &[
    ("mypy", "mypy src", |c, n| format!("src/m.py:{n}: error: Problem {c:02}  [code-{c:02}]"), "Found 1 error in 1 file", |c, _| format!("code-{c:02}")),
    ("ruff", "ruff check .", |c, n| format!("src/a.py:{n}:1: X{c:03} Problem {c:02}"), "Found 1 error.", |c, _| format!("X{c:03}")),
    ("oxlint", "oxlint src", |c, n| format!("  x eslint(rule-{c:02}): Problem\n   ,-[src/a.js:{n}:3]\n 3 | foo();\n   `----"), "Found 1 error.", |c, _| format!("rule-{c:02}")),
    ("biome", "biome check src", |c, n| format!("src/a.ts:{n}:7 lint/correctness/rule{c:02}  ━━━━━━━━━━━━━━━━\n  × Problem\n"), "Found 1 error.", |c, _| format!("rule{c:02}")),
    ("rubocop", "rubocop", |c, n| format!("app/a.rb:{n}:5: C: Style/Cop{c:02}: Problem"), "1 file inspected, 1 offense detected", |c, _| format!("Cop{c:02}")),
    ("rubocop json", "rubocop --format json", |c, n| format!(r#"{{"severity":"convention","message":"Problem","cop_name":"Style/Cop{c:02}","correctable":false,"location":{{"start_line":{n},"start_column":5}}}}"#), "", |c, _| format!("Cop{c:02}")),
    ("sqlfluff json", "sqlfluff lint --format json", |c, n| format!(r#"{{"line_no":{n},"line_pos":1,"code":"LT{c:02}","description":"Problem"}}"#), "", |c, _| format!("LT{c:02}")),
    ("phpstan json", "phpstan analyse --error-format=json", |c, n| format!(r#"{{"message":"Problem","line":{n},"identifier":"id.name{c:02}"}}"#), "", |c, _| format!("id.name{c:02}")),
    ("golangci-lint", "golangci-lint run", |c, n| format!("a.go:{n}:2: Problem (lint{c:02})"), "1 issues.", |c, _| format!("lint{c:02}")),
    ("cargo", "cargo build", |c, n| format!("error[E{c:04}]: Problem {c:02}\n --> src/lib.rs:{n}:5\n  |\n3 | x\n  | ^\n"), "error: could not compile `x` due to 1 previous error", |c, _| format!("E{c:04}")),
    ("gcc", "make", |c, n| format!("a.c:{n}:5: error: Problem {c:02}\n   12 |   foo(bar_baz_qux);\n      |       ^~~~~~~~~~~\n"), "make: *** [all] Error 1", |c, _| format!("Problem {c:02}")),
    ("dotnet", "dotnet build", |c, n| format!("Program.cs({n},5): error CS{:04}: Problem [p.csproj]", 1000 + c), "Build FAILED.", |c, _| format!("CS{:04}", 1000 + c)),
    ("mvn", "mvn compile", |c, n| format!("[ERROR] /src/A.java:[{n},5] Problem {c:02}"), "[INFO] BUILD FAILURE", |c, _| format!("Problem {c:02}")),
    ("go build", "go build ./...", |c, n| format!("./a.go:{n}:2: Problem {c:02}"), "", |c, _| format!("Problem {c:02}")),
    ("tsc", "tsc --noEmit", |c, n| format!("a.ts({n},1): error TS{:04}: Problem {c:02}", 2000 + c), "", |c, _| format!("TS{:04}", 2000 + c)),
    ("eslint", "eslint src", |c, n| format!("  {n}:1  error  Problem {c:02}  rule-{c:02}"), "✖ 1 problem (1 error, 0 warnings)", |c, _| format!("rule-{c:02}")),
];

const CLASSES: usize = 70;

/// Class `c` occurs `1 + c % 4` times, in class order.
fn input(row: Row, tail: &str, json: Option<&str>) -> Vec<String> {
    let mut n = 0;
    let mut rows = vec![];
    for c in 0..CLASSES { for _ in 0..1 + c % 4 { n += 1; rows.push(row(c, n)); } }
    let text = match json {
        Some("rubocop") => format!(r#"{{"files":[{{"path":"app/a.rb","offenses":[{}]}}]}}"#, rows.join(",")),
        Some("sqlfluff") => format!(r#"[{{"filepath":"m.sql","violations":[{}]}}]"#, rows.join(",")),
        Some("phpstan") => format!(r#"{{"totals":{{"errors":0,"file_errors":{n}}},"files":{{"app/A.php":{{"errors":{n},"messages":[{}]}}}},"errors":[]}}"#, rows.join(",")),
        _ => rows.join("\n") + "\n" + tail,
    };
    text.lines().map(String::from).collect()
}

fn json_kind(name: &str) -> Option<&'static str> {
    match name { "rubocop json" => Some("rubocop"), "sqlfluff json" => Some("sqlfluff"), "phpstan json" => Some("phpstan"), _ => None }
}

#[test]
fn no_diagnostic_class_is_dropped_by_a_dedicated_view() {
    let mut lost = vec![];
    for (name, cmd, row, tail, marker) in CASES {
        let lines = input(*row, tail, json_kind(name));
        let Some(view) = crate::cmdviews::view(cmd, &lines) else { lost.push(format!("{name}: the view did not take the output")); continue };
        // the whole pipeline too: declarative filters run before the views
        let shown = crate::shell::preview(cmd, &lines.join("\n"), 1, 700);
        for (what, out) in [("view", &view), ("run", &shown)] {
            let missing: Vec<String> = (0..CLASSES).map(|c| marker(c, 0)).filter(|m| !out.contains(m.as_str())).collect();
            if !missing.is_empty() { lost.push(format!("{name} ({what}): {} of {CLASSES} classes missing, e.g. {}", missing.len(), missing[0])); }
        }
    }
    assert!(lost.is_empty(), "classes lost silently:\n{}", lost.join("\n"));
}

#[test]
fn a_cut_names_what_it_cut() {
    use crate::diag::cap_classes as cut;
    let rows: Vec<(String, String)> = (0..10).map(|i| (format!("r{}", i % 3), format!("row {i}"))).collect();
    // the first row of every class, then the earliest others; what is left is counted per class
    assert_eq!(cut(rows, 4), ["row 0", "row 1", "row 2", "row 3", "... +6 more (r1 x2, r2 x2, r0 x2)"]);
    let many: Vec<(String, String)> = (0..8).map(|i| (format!("c{i}"), format!("row {i}"))).collect();
    assert_eq!(cut(many, 3).len(), 9, "more classes than rows: every class keeps its first row");
    assert_eq!(cut(vec![("a".into(), "x".into())], 4), ["x"]);
}
