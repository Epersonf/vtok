//! Files the grammars cannot fully parse: unparsed regions are carried verbatim, the rest is still compacted, and the
//! decoded text always matches the source (lexically: same strings, same code outside comments and whitespace).
use crate::*;
use std::collections::HashMap;
use vtok_core::lang::EncodeOpts;
use vtok_core::language::Language;

fn lossless(l: &'static Gen, src: &str) -> String {
    let e = l.encode(src, &EncodeOpts::default()).unwrap_or_else(|e| panic!("{}: refused: {e}", l.0.name));
    let inv: HashMap<String, String> = e.table.iter().map(|(o, a)| (a.clone(), o.clone())).collect();
    let back = l.decode(&e.vtok, &inv, &HashMap::new()).unwrap();
    assert_eq!(lex::proof(l.0, &back), lex::proof(l.0, src), "{}: decode differs\n{}", l.0.name, e.vtok);
    assert!(e.vtok.len() < src.len(), "{}: not smaller\n{}", l.0.name, e.vtok);
    e.vtok
}

#[test]
fn macro_annotated_win32_declarations() {
    let src = "#include <windows.h>\n\n// window procedure\nLRESULT CALLBACK\tWndProc(HWND, UINT, WPARAM, LPARAM);\n\nint   main ( void )\n{\n    int   total = 0 ;   // sum\n    return   total ;\n}\n";
    let v = lossless(&CPP, src);
    assert!(v.contains("LRESULT CALLBACK\tWndProc") && v.contains("int main(void){int total=0;return total;}") && !v.contains("// sum") && !v.contains("window procedure"), "{v}");
}

#[test]
fn compound_literal_macros_in_c() {
    let src = "#include \"global.h\"\n\n/* make a node */\nnode_t *make(void) {\n    node_t *n = ALLOC_INIT(node_t, {.id     = 0,\n                                    .name   = \"x\"});\n    return   n;\n}\n\nint   other ( int   a ) { return a  +  1; }\n";
    let v = lossless(&C, src);
    assert!(v.contains(".id     = 0") && v.contains("int other(int a){return a+1;}") && !v.contains("make a node"), "{v}");
}

#[test]
fn strings_continued_with_backslash_stay_exact() {
    let src = "char *shader = \"\\\nfloat hash(float3 k) { \\\n\t/* not a comment */ return k.x; \\\n}\";\n\nint   used ( int   x ) { return x ; }\n";
    let v = lossless(&C, src);
    assert!(v.contains("float hash(float3 k) { \\\n\t/* not a comment */ return k.x;"), "{v}");
}

#[test]
fn postgres_ddl_the_grammar_lacks() {
    let src = "-- events\ncreate table public.events (\n  id bigint primary key,\n  body   text   not null\n);\n\nalter table public.events enable row level security;\n\ncreate policy \"read own\" on public.events for select using (auth.uid() = owner);\n\ncomment on table public.events is 'kept';\n\ncreate function f() returns int language plpgsql as $$\nbegin\n  return 1; -- one\nend;\n$$;\n";
    let v = lossless(&SQL, src);
    assert!(v.contains("create table public.events(id bigint primary key,body text not null);"), "{v}");
    assert!(v.contains("alter table public.events enable row level security;") && v.contains("comment on table public.events is'kept'"), "{v}");
    assert!(v.contains("create policy \"read own\" on public.events") && v.contains("$$") && !v.contains("-- events"), "{v}");
}

#[test]
fn sql_comment_on_statements_are_code_not_comments() {
    let src = "create table t (a int);\ncomment on column t.a is 'the a';\n";
    let e = SQL.encode(src, &EncodeOpts::default()).unwrap();
    assert!(e.vtok.contains("comment on column t.a is'the a'"), "{}", e.vtok);
}

#[test]
fn html_with_stray_markup_and_comments_between_text() {
    let src = "<!DOCTYPE html>\n<html>\n<body>\n  <ul>\n    <li>Select File->New Project</li>\n  </ul>\n  <p>\n  Updated:\n  <!-- stamp -->\n  $Date: 2009 $\n  </p>\n</body>\n</html>\n";
    let v = lossless(&HTML, src);
    assert!(v.contains("File->New Project") && v.contains("$Date: 2009 $"), "{v}");
}

#[test]
fn csharp_directives_after_a_byte_order_mark() {
    let src = "\u{feff}#if !UNITY_2018\nusing UnityEditor;\n\nnamespace A\n{\n    // tools\n    public static class B\n    {\n        public static void C() { }\n    }\n}\n#endif\n";
    let e = CSHARP.encode(src, &EncodeOpts::default()).unwrap_or_else(|e| panic!("{e}"));
    assert!(e.vtok.contains("#if!UNITY_2018\nusing UnityEditor;\n") && e.vtok.ends_with("\n#endif\n"), "{}", e.vtok);
}

#[test]
fn mostly_unparseable_files_are_still_refused() {
    assert!(JAVA.encode("class {", &EncodeOpts::default()).is_err());
    assert!(C.encode("@@@ ### $$$ !!! ((( ]]] }}}\n", &EncodeOpts::default()).is_err());
    assert!(SQL.encode("@@@ ### ((( ]]] !!! ;\n", &EncodeOpts::default()).is_err());
}
