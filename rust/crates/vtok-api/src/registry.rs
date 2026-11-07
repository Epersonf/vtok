//! The installed language plugins and how to find one by name or by file path.
use crate::{Language, Result, VtokError};

static PY: vtok_py::Python = vtok_py::Python;
static TS: vtok_ts::TypeScript = vtok_ts::TypeScript;
static RS: vtok_rs::Rust = vtok_rs::Rust;

pub fn languages() -> Vec<&'static dyn Language> {
    use vtok_gen::*;
    vec![&PY, &TS, &RS, &JAVA, &GO, &C, &CPP, &CSHARP, &DART, &CSS, &HTML, &SQL]
}

pub fn language(name: &str) -> Result<&'static dyn Language> {
    languages().into_iter().find(|l| l.name() == name || (name == "ts" && l.name() == "typescript") || (name == "py" && l.name() == "python") || (name == "rs" && l.name() == "rust") || (name == "c++" && l.name() == "cpp") || (name == "cs" && l.name() == "csharp"))
        .ok_or_else(|| VtokError::new(format!("unknown language {name:?}; installed: {}", languages().iter().map(|l| l.name()).collect::<Vec<_>>().join(", "))))
}

pub fn language_for_path(path: &str) -> Result<&'static dyn Language> {
    languages().into_iter().find(|l| l.extensions().iter().any(|e| path.ends_with(e)))
        .ok_or_else(|| VtokError::new(format!("no language plugin for {path:?}")))
}

pub fn normalize_newlines(s: &str) -> String { s.replace("\r\n", "\n") }
