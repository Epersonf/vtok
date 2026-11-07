//! Identifier occurrences of a source, for name-based search.
use crate::{language, normalize_newlines, Result, VtokError};

/// Every identifier occurrence in the source as (name, 1-based line), from the grammar's leaf tokens
/// (comments and string contents are not identifiers). Used for name-based reference search.
pub fn idents(lang: &str, source: &str) -> Result<Vec<(String, usize)>> {
    Ok(idents_dot(lang, source)?.into_iter().map(|(n, l, _, _)| (n, l)).collect())
}

/// Like `idents`, plus whether the identifier directly follows a `.` (method or field access) and whether it is directly
/// followed by `(` (call form).
pub fn idents_dot(lang: &str, source: &str) -> Result<Vec<(String, usize, bool, bool)>> {
    let l = language(lang)?;
    let src = normalize_newlines(source);
    let spans = l.identifier_spans(&src).ok_or_else(|| VtokError::new("cannot tokenize this source (syntax error?)"))?;
    let mut line_starts = vec![0usize];
    for (i, b) in src.bytes().enumerate() { if b == b'\n' { line_starts.push(i + 1); } }
    let mut out = vec![];
    for (a, b) in spans {
        let Some(t) = src.get(a..b) else { continue };
        let mut cs = t.chars();
        if !cs.next().map_or(false, |c| c.is_alphabetic() || c == '_' || c == '$') || !t.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '$') { continue; }
        let line = line_starts.partition_point(|&s| s <= a);
        let dot = src.get(..a).map_or(false, |h| h.trim_end().ends_with('.'));
        let call = src.get(b..).map_or(false, |t| t.trim_start_matches(|c: char| c == ' ' || c == '\t').starts_with('(')) || src.get(b..).map_or(false, |t| t.starts_with("::<"));
        out.push((t.to_string(), line, dot, call));
    }
    Ok(out)
}
