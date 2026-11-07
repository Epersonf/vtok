//! Generic tree-sitter language plugins: HTML, CSS, SQL, Java, Dart, C++, C#, Go and C.
//! One implementation (`Gen`) driven by a per-grammar `Spec`; the encoder always verifies that the minified text parses
//! to the same tree (comments aside), so a file that cannot be minified safely is refused instead of corrupted.
mod emit;
mod plugin;
pub mod spec;
mod tree;

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
