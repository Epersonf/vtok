//! Skeleton, lean and outline of the non-code languages (CSS, HTML, SQL), and outline / skeleton fixes for C and Dart.
use crate::*;
use vtok_core::lang::{EncodeOpts, View};
use vtok_core::language::Language;

fn view(l: &'static Gen, src: &str, v: View) -> String { l.encode(src, &EncodeOpts { view: v, ..Default::default() }).unwrap().vtok }

fn outline(l: &'static Gen, src: &str) -> Vec<String> { l.units(src, 1).unwrap().into_iter().map(|u| u.name).collect() }

const CSS_SRC: &str = "/* theme */\n:root { --bg: #fff; --fg: #000; color: red; }\n.card:hover, a > b { margin: 0; -webkit-transition: x 1s; transition: x 1s; }\n@media (max-width: 600px) { .card { padding: 0; } }\n@keyframes spin { from { opacity: 0; } to { opacity: 1; } }\n@font-face { font-family: X; src: url(data:font/woff2;base64,AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA); }\n";

#[test]
fn css_outline_names_selectors_and_at_rules() {
    let o = outline(&CSS, CSS_SRC);
    assert_eq!(o, [":root", ".card:hover,a > b", "@media (max-width: 600px)", ".card", "@keyframes spin", "@font-face"], "{o:?}");
}

#[test]
fn css_skeleton_counts_declarations_and_lean_drops_noise() {
    let s = view(&CSS, CSS_SRC, View::Skeleton);
    assert!(s.contains(":root{--bg,--fg,+1}") && s.contains(".card:hover,a > b{3}") && s.contains("@media (max-width: 600px){.card{1}}") && s.contains("@keyframes spin{from,to}"), "{s}");
    let l = view(&CSS, CSS_SRC, View::Lean);
    let f = view(&CSS, CSS_SRC, View::Full);
    assert!(f.contains("-webkit-transition") && !l.contains("-webkit-transition") && l.contains("transition:x 1s"), "{l}");
    assert!(l.contains("url(data:…") && !l.contains("AAAA") && l.len() < f.len() && s.len() < l.len(), "{l}");
}

const HTML_SRC: &str = "<!DOCTYPE html>\n<html>\n<head>\n  <title>Docs</title>\n  <link rel=\"stylesheet\" href=\"site.css\">\n  <script src=\"app.js\"></script>\n  <style>\n  body { margin: 0; padding: 0; color: #222; background: #fafafa; font-family: sans-serif; line-height: 1.5; }\n  .nav { display: flex; gap: 1em; } .nav a { color: inherit; text-decoration: none; } main { max-width: 60em; }\n  </style>\n</head>\n<body class=\"page\">\n  <!-- top -->\n  <nav id=\"top\" class=\"nav\"><a href=\"/\">Home</a><a href=\"/api\">API</a></nav>\n  <main>\n    <h1>Getting <b>started</b></h1>\n    <p>Some long text that the skeleton drops.</p>\n    <form action=\"/q\"><input type=\"text\" name=\"q\"></form>\n  </main>\n</body>\n</html>\n";

#[test]
fn html_outline_lists_landmarks() {
    let o = outline(&HTML, HTML_SRC);
    for want in ["html", "head", "title Docs", "link[href=site.css]", "script[src=app.js]", "body.page", "nav#top.nav", "main", "h1 Getting started", "form"] {
        assert!(o.iter().any(|x| x == want), "{want} missing: {o:?}");
    }
}

#[test]
fn html_skeleton_keeps_structure_and_lean_elides_inline_bodies() {
    let s = view(&HTML, HTML_SRC, View::Skeleton);
    assert!(s.contains("<nav id=top class=nav><a href=/>×2</nav>") || s.contains("<nav id=top class=nav><a href=/><a href=/api></nav>"), "{s}");
    assert!(s.contains("<h1>Getting started</h1>") && s.contains("<input type=text name=q>") && !s.contains("long text"), "{s}");
    let l = view(&HTML, HTML_SRC, View::Lean);
    assert!(l.contains("lines,") && !l.contains("text-decoration") && l.contains("long text") && !l.contains("top -->"), "{l}");
}

const SQL_SRC: &str = "-- schema\ncreate table if not exists public.users (\n  id bigint primary key,\n  org uuid references public.orgs(id),\n  name text not null default '',\n  constraint u unique (name)\n);\ncreate unique index users_name on public.users (name);\ncreate or replace function public.touch() returns trigger language plpgsql as $$\nbegin\n  new.at = now();\n  return new;\nend;\n$$;\ncreate view public.v as select id from public.users;\nalter table public.users enable row level security;\ninsert into public.users (id, name) values (1, 'a'), (2, 'b'), (3, 'c'), (4, 'd'), (5, 'e');\n";

#[test]
fn sql_outline_names_the_objects() {
    let o = outline(&SQL, SQL_SRC);
    assert_eq!(o, ["create table public.users", "create index users_name on public.users", "create function public.touch", "create view public.v", "alter table public.users", "insert into public.users (5 rows)"], "{o:?}");
}

#[test]
fn sql_skeleton_keeps_heads_and_lean_cuts_rows() {
    let s = view(&SQL, SQL_SRC, View::Skeleton);
    assert!(s.contains("create table if not exists public.users(id bigint pk,org uuid ->public.orgs,name text)"), "{s}");
    assert!(s.contains("returns trigger language plpgsql as $$…6 lines$$") && s.contains("create view public.v as …") && s.contains("values …5 rows"), "{s}");
    let l = view(&SQL, SQL_SRC, View::Lean);
    assert!(l.contains("(3,'c')/* +2 rows */;") && !l.contains("'e'") && l.contains("new.at = now();") && !l.contains("-- schema"), "{l}");
}

#[test]
fn c_skeleton_drops_function_bodies_and_outline_sees_inside_guards() {
    let src = "#ifndef A_H\n#define A_H\nstatic int count;\nint total(int n) {\n    int acc = 0;\n    for (int i = 0; i < n; i++) acc += i;\n    return acc;\n}\n#endif\n";
    let s = view(&C, src, View::Skeleton);
    assert!(s.contains("int total(int n){}") && !s.contains("acc"), "{s}");
    let o = outline(&C, src);
    assert!(o.contains(&"count".to_string()) && o.contains(&"total".to_string()), "{o:?}");
}

#[test]
fn dart_outline_names_variables_not_their_types() {
    let o = outline(&DART, "const double kWidth = 1.0;\nclass A {\n  final double size;\n  double get half => size / 2;\n  A(this.size);\n}\n");
    assert!(o.contains(&"kWidth".to_string()) && !o.contains(&"double".to_string()), "{o:?}");
}
