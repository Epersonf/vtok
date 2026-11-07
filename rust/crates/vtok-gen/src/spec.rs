//! One `Spec` per grammar: everything that differs between the generic plugin's languages.
use tree_sitter::Language as TsLanguage;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode { Code, Html, Css }

pub struct Spec {
    pub name: &'static str,
    pub exts: &'static [&'static str],
    pub grammar: fn() -> TsLanguage,
    pub mode: Mode,
    /// Rename identifiers through the alias table (off where names are case-insensitive or are not code identifiers).
    pub alias: bool,
    /// A line break between tokens is meaningful (Go ends statements at newlines without a token for it).
    pub nl: bool,
    /// Preprocessor directives end at the line end (C, C++): keep line breaks next to `#` lines.
    pub dir: bool,
    /// Builtin names the alias stream must never produce (types and functions that are identifiers in the grammar).
    pub words: &'static str,
    /// Text around member chunks so they parse on their own; None = this language is chunked at top level only.
    pub wrap: Option<(&'static str, &'static str)>,
    /// Split into top-level statements when the file does not parse as a whole (SQL): each statement is parsed on
    /// its own, so an unparsed statement stays a verbatim chunk instead of derailing the rest.
    pub stmts: bool,
    pub prompt: &'static str,
}

macro_rules! prompt {
    ($n:expr) => {
        concat!("Vtok-", $n, ": minified ", $n, ". One top-level item per line, no comments, no optional spaces; line breaks only where ", $n, " needs them.\nNames in the alias table (alias=original) are used shortened; new names are written in full.\n")
    };
}

fn html() -> TsLanguage { tree_sitter_html::LANGUAGE.into() }
fn css() -> TsLanguage { tree_sitter_css::LANGUAGE.into() }
fn sql() -> TsLanguage { tree_sitter_sequel::LANGUAGE.into() }
fn java() -> TsLanguage { tree_sitter_java::LANGUAGE.into() }
fn dart() -> TsLanguage { tree_sitter_dart::LANGUAGE.into() }
fn cpp() -> TsLanguage { tree_sitter_cpp::LANGUAGE.into() }
fn csharp() -> TsLanguage { tree_sitter_c_sharp::LANGUAGE.into() }
fn go() -> TsLanguage { tree_sitter_go::LANGUAGE.into() }
fn c() -> TsLanguage { tree_sitter_c::LANGUAGE.into() }

pub static HTML: Spec = Spec { name: "html", exts: &[".html", ".htm"], grammar: html, mode: Mode::Html, alias: false, nl: false, dir: false, words: "", wrap: None, stmts: false, prompt: prompt!("HTML") };
pub static CSS: Spec = Spec { name: "css", exts: &[".css"], grammar: css, mode: Mode::Css, alias: false, nl: false, dir: false, words: "", wrap: None, stmts: false, prompt: prompt!("CSS") };
pub static SQL: Spec = Spec { name: "sql", exts: &[".sql"], grammar: sql, mode: Mode::Code, alias: false, nl: false, dir: false, words: "", wrap: None, stmts: true, prompt: prompt!("SQL") };
pub static JAVA: Spec = Spec {
    name: "java", exts: &[".java"], grammar: java, mode: Mode::Code, alias: true, nl: false, dir: false,
    words: "String Object Integer Long Double Boolean List Map Set Math System out err Override Exception Class Thread Optional Arrays Collections length size get put add main args",
    wrap: Some(("class __vtok_wrap__ {", "}")), stmts: false, prompt: prompt!("Java"),
};
pub static DART: Spec = Spec {
    name: "dart", exts: &[".dart"], grammar: dart, mode: Mode::Code, alias: true, nl: false, dir: false,
    words: "String int double num bool List Map Set Future Stream Object dynamic void print override main Widget State BuildContext",
    wrap: Some(("class __vtok_wrap__ {", "}")), stmts: false, prompt: prompt!("Dart"),
};
pub static CPP: Spec = Spec {
    name: "cpp", exts: &[".cpp", ".cc", ".cxx", ".hpp", ".hh", ".hxx"], grammar: cpp, mode: Mode::Code, alias: true, nl: false, dir: true,
    words: "std string vector map set size_t int8_t int16_t int32_t int64_t uint8_t uint16_t uint32_t uint64_t cout cin endl main nullptr NULL printf",
    wrap: Some(("struct __vtok_wrap__ {", "};")), stmts: false, prompt: prompt!("C++"),
};
pub static CSHARP: Spec = Spec {
    name: "csharp", exts: &[".cs"], grammar: csharp, mode: Mode::Code, alias: true, nl: false, dir: true,
    words: "String Object Int32 Int64 List Dictionary Task Console System Math Exception Main args value var async await get set",
    wrap: Some(("class __vtok_wrap__ {", "}")), stmts: false, prompt: prompt!("C#"),
};
pub static GO: Spec = Spec {
    name: "go", exts: &[".go"], grammar: go, mode: Mode::Code, alias: true, nl: true, dir: false,
    words: "string int int8 int16 int32 int64 uint uint8 uint16 uint32 uint64 uintptr float32 float64 bool byte rune error any nil true false iota make len cap new append copy delete panic recover print println close fmt main init",
    wrap: None, stmts: false, prompt: prompt!("Go"),
};
pub static C: Spec = Spec {
    name: "c", exts: &[".c", ".h"], grammar: c, mode: Mode::Code, alias: true, nl: false, dir: true,
    words: "size_t int8_t int16_t int32_t int64_t uint8_t uint16_t uint32_t uint64_t FILE NULL main printf malloc free memcpy strlen",
    wrap: None, stmts: false, prompt: prompt!("C"),
};
