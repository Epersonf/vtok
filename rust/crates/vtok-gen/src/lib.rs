//! Generic tree-sitter language plugins: HTML, CSS, SQL, Java, Dart, C++, C#, Go and C.
//! One implementation (`Gen`) driven by a per-grammar `Spec`; the encoder always verifies that the minified text parses
//! to the same tree (comments aside), so a file that cannot be minified safely is refused instead of corrupted.
//! Regions the grammar cannot parse are carried verbatim and the rest is still minified (`emit`, `verbatim`, `lex`;
//! SQL statement by statement, `stmts`); only a file that is mostly unparseable, or whose proof fails, is refused.
//!
//! Files: `plugin` (the `Language` impl), `gen` (verification, units, aliases), `emit` + `verbatim` + `space` (the
//! minifier), `tree` (parse helpers, canonical dump), `lex` (string/comment lexer and lexical proof), `stmts` (SQL
//! statements), `markup` + `css_view` + `html_view` + `sql_view` + `sql_words` (views and outline of CSS/HTML/SQL).
mod css_view;
mod emit;
mod gen;
mod html_view;
mod lex;
mod markup;
mod plugin;
mod space;
pub mod spec;
mod sql_view;
mod sql_words;
mod stmts;
mod tree;
mod verbatim;

pub use plugin::Gen;

pub static HTML: Gen = Gen(&spec::HTML);
pub static CSS: Gen = Gen(&spec::CSS);
pub static SQL: Gen = Gen(&spec::SQL);
pub static JAVA: Gen = Gen(&spec::JAVA);
pub static DART: Gen = Gen(&spec::DART);
pub static CPP: Gen = Gen(&spec::CPP);
pub static CSHARP: Gen = Gen(&spec::CSHARP);
pub static GO: Gen = Gen(&spec::GO);
pub static C: Gen = Gen(&spec::C);

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_fallback;
#[cfg(test)]
mod tests_views;
