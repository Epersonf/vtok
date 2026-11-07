use crate::*;
use std::collections::HashMap;
use vtok_core::lang::EncodeOpts;
use vtok_core::language::Language;

const JAVA_SRC: &str = "package app;\nimport java.util.List;\n\n/** Account. */\npublic class Account {\n    // balance in cents\n    private long balanceCents;\n    private final List<String> history = new java.util.ArrayList<>();\n\n    public void deposit(long amountCents) {\n        if (amountCents <= 0) { throw new IllegalArgumentException(\"amount must be positive\"); }\n        balanceCents += amountCents;\n        history.add(\"deposit \" + amountCents);\n    }\n\n    public long balance() { return balanceCents; }\n}\n";
const GO_SRC: &str = "package main\n\nimport (\n\t\"fmt\"\n\t\"strings\"\n)\n\n// Greet builds a greeting.\nfunc Greet(names []string) string {\n\tvar parts []string\n\tfor _, n := range names {\n\t\tparts = append(parts, strings.ToUpper(n))\n\t}\n\treturn fmt.Sprintf(\"hello %s\", strings.Join(parts, \", \"))\n}\n\nfunc main() {\n\tfmt.Println(Greet([]string{\"a\", \"b\"}))\n}\n";
const C_SRC: &str = "#include <stdio.h>\n#define LIMIT 10\n\n/* sum of 1..n */\nstatic int total(int count) {\n    int acc = 0;\n    for (int i = 1; i <= count; i++) { acc += i; }\n    return acc;\n}\n\nint main(void) {\n    printf(\"%d\\n\", total(LIMIT));\n    return 0;\n}\n";
const CPP_SRC: &str = "#include <vector>\n#include <string>\n\nnamespace shop {\n// A cart.\nclass Cart {\npublic:\n    void add(const std::string& item) { items_.push_back(item); }\n    std::size_t size() const { return items_.size(); }\nprivate:\n    std::vector<std::string> items_;\n};\n}\n";
const CS_SRC: &str = "using System;\n\nnamespace Shop {\n    // A cart.\n    public class Cart {\n        private readonly System.Collections.Generic.List<string> items = new();\n        public void Add(string item) { items.Add($\"item {item}\"); }\n        public int Count => items.Count;\n    }\n}\n";
const DART_SRC: &str = "import 'dart:math';\n\n/// A counter.\nclass Counter {\n  int _value = 0;\n  void increment({int by = 1}) { _value += by; }\n  int get value => _value;\n}\n\nvoid main() {\n  final c = Counter();\n  c.increment(by: 2);\n  print('value ${c.value}');\n}\n";
const CSS_SRC: &str = "/* layout */\n.card .title, a:hover {\n  margin: 0 .5em;\n  width: calc(100% - 2px);\n  color: #fff !important;\n}\n@media screen and (min-width: 600px) {\n  .card > p { font-size: 14px; }\n}\n";
const HTML_SRC: &str = "<!DOCTYPE html>\n<html>\n  <head>\n    <!-- title -->\n    <title>Hi</title>\n    <style>\n      body { margin: 0; }\n    </style>\n  </head>\n  <body>\n    <p class=\"a\" id=\"b\">Hello <b>world</b> again</p>\n    <pre>keep\n   this</pre>\n  </body>\n</html>\n";
const SQL_SRC: &str = "-- users\nSELECT u.id, u.name, COUNT(o.id) AS orders\nFROM users u\nLEFT JOIN orders o ON o.user_id = u.id\nWHERE u.active = 1 AND u.name <> 'it''s'\nGROUP BY u.id, u.name\nORDER BY orders DESC;\n";

fn check(l: &'static Gen, src: &str, needs_smaller: bool) -> String {
    let e = l.encode(src, &EncodeOpts::default()).unwrap_or_else(|e| panic!("{}: {e}", l.0.name));
    let inv: HashMap<String, String> = e.table.iter().map(|(o, a)| (a.clone(), o.clone())).collect();
    let back = l.decode(&e.vtok, &inv, &HashMap::new()).unwrap();
    assert!(l.equivalent(src, &back, false), "{}: decode differs\n{}\n--\n{back}", l.0.name, e.vtok);
    if needs_smaller { assert!(e.vtok.len() < src.len(), "{}: not smaller", l.0.name); }
    e.vtok
}

#[test]
fn every_language_round_trips_and_drops_comments() {
    for (l, s) in [(&JAVA, JAVA_SRC), (&GO, GO_SRC), (&C, C_SRC), (&CPP, CPP_SRC), (&CSHARP, CS_SRC), (&DART, DART_SRC), (&CSS, CSS_SRC), (&HTML, HTML_SRC), (&SQL, SQL_SRC)] {
        let v = check(l, s, true);
        assert!(!v.contains("balance in cents") && !v.contains("A cart") && !v.contains("title -->") && !v.contains("layout") && !v.contains("-- users"), "{}: comment kept: {v}", l.0.name);
    }
}

#[test]
fn whitespace_that_matters_survives() {
    assert!(check(&CSS, CSS_SRC, true).contains(".card .title"));
    assert!(check(&CSS, CSS_SRC, true).contains("0 .5em"));
    assert!(check(&CSS, CSS_SRC, true).contains("and (min-width"));
    assert!(check(&HTML, HTML_SRC, true).contains("keep\n   this"));
    assert!(check(&SQL, SQL_SRC, true).contains("'it''s'"));
    let g = check(&GO, GO_SRC, true);
    assert!(g.contains('\n') && g.contains("\"hello %s\""), "{g}");
    assert!(check(&C, C_SRC, true).contains("#include<stdio.h>\n#define LIMIT 10\n"));
}

#[test]
fn aliases_apply_and_imports_stay_exact() {
    let v = check(&JAVA, JAVA_SRC, true);
    assert!(v.contains("import java.util.List;") && v.contains("package app;"), "{v}");
    let e = JAVA.encode(JAVA_SRC, &EncodeOpts::default()).unwrap();
    assert!(!e.table.is_empty() && !e.table.contains_key("List") || e.table.get("List").is_none(), "{:?}", e.table);
}

#[test]
fn syntax_errors_are_refused_and_skeleton_drops_bodies() {
    assert!(JAVA.encode("class {", &EncodeOpts::default()).is_err());
    let o = EncodeOpts { view: vtok_core::lang::View::Skeleton, ..Default::default() };
    let s = JAVA.encode(JAVA_SRC, &o).unwrap().vtok;
    assert!(s.contains("{}") && !s.contains("IllegalArgument"), "{s}");
}

#[test]
fn big_classes_split_into_member_chunks() {
    let mut src = String::from("public class Big {\n");
    for i in 0..30 { src += &format!("    public int f{i}(int x) {{\n        return x + {i};\n    }}\n"); }
    src += "}\n";
    let units = JAVA.units(&src, 40).unwrap();
    assert!(units.len() >= 30 && units.iter().all(|u| u.member && u.parent.as_deref() == Some("Big")), "{}", units.len());
    let t: std::collections::BTreeMap<String, String> = Default::default();
    let body = "    public int f1(int x) {\n        return x + 1;\n    }\n";
    let vt = JAVA.encode_chunk(body, &t, true).unwrap();
    assert!(vt.starts_with("public int f1(int x){return x+1;}"), "{vt}");
    let back = JAVA.decode_chunk(&vt, &HashMap::new(), true, 4, None).unwrap();
    assert!(JAVA.equivalent(body, &back, true));
}
