//! The plain-text side of the Vtok tools. A file with no Vtok notation, or one where the compact form would not save enough
//! (`min-gain`), comes back as it is, with its rev; `vtok_edit` then changes it by exact text (`replace`). The agent never
//! has to work out beforehand whether Vtok pays, and never has to switch to another tool when it does not.
//!
//! The decision is made on what would really be returned: the compact answer and the plain answer are both built, and the
//! compact one is kept only if it is at least `min-gain` percent smaller. Headers stay short because both sides pay them.
use super::args::{arg_str, fnv};
use serde_json::Value;
use vtok_api::{language_for_path, normalize_newlines, ntok, symbols};

/// A plain read stops here: a bigger file needs a range or a view (`vtok run -- cat FILE` gives data files a table).
const PLAIN_CAP_TOKENS: usize = 10_000;
/// A data file from this size on is tried as a table first.
const TABLE_FROM_TOKENS: usize = 2_000;

/// Whether the answer `shown` is at least `gain` percent smaller than the answer `plain`.
pub(super) fn pays(shown: &str, plain: &str, gain: usize) -> bool {
    ntok(shown) * 100 < ntok(plain) * (100 - gain)
}

/// The `min-gain` setting that applies to the file (a `.vtok.json` next to it wins over the user file).
pub(super) fn gain_near(path: &str) -> usize {
    crate::config::min_gain(Some(std::path::Path::new(path)))
}

/// Whether the path has a Vtok notation (a language plugin).
pub(super) fn is_source(path: &str) -> bool {
    language_for_path(path).is_ok()
}

fn read_text(path: &str) -> Result<(String, bool), String> {
    let raw = std::fs::read_to_string(path).map_err(|e| format!("cannot read {path}: {e}"))?;
    Ok((normalize_newlines(&raw), raw.contains("\r\n")))
}

/// `vtok_read` of a file with no Vtok notation (Markdown, TOML, JSON, text...): the text and the rev `vtok_edit` needs. A big
/// CSV/JSON/YAML file comes back as the table view `vtok run -- cat` gives it.
pub(super) fn read(path: &str) -> Result<String, String> {
    let (src, _) = read_text(path)?;
    let (tokens, rev) = (ntok(&src), fnv(&src));
    let ext = path.rsplit_once('.').map(|x| x.1.to_ascii_lowercase());
    if tokens > TABLE_FROM_TOKENS && ext.as_deref().is_some_and(crate::data::is_data_ext) {
        let lines: Vec<String> = src.lines().map(String::from).collect();
        if let Some(view) = crate::data::view(&lines, ext.as_deref()) {
            let shown = ntok(&view);
            crate::usage::record("read", tokens, shown, path);
            return Ok(format!("# {path} rev={rev} table view of a data file, {tokens} tokens -> {shown}; odd records kept as they are\n---\n{view}"));
        }
    }
    if tokens > PLAIN_CAP_TOKENS {
        return Err(format!("{path} has {tokens} tokens, more than a plain read returns ({PLAIN_CAP_TOKENS}): read a range with your normal read tool"));
    }
    crate::usage::record("read", tokens, tokens, path);
    Ok(format!("# {path} rev={rev} plain text, {tokens} tokens\n---\n{src}"))
}

/// What `vtok_read` returns for a source file when the compact form would not save enough.
pub(super) fn plain_read(path: &str, src: &str) -> String {
    format!("# {path} rev={} plain text (Vtok saves less than min-gain here)\n---\n{src}", fnv(src))
}

/// What `vtok_chunk` returns for a symbol when the compact form would not save enough.
pub(super) fn plain_chunk(path: &str, id: &str, src: &str, text: &str, lines: (usize, usize)) -> String {
    format!("# {path} chunk {id} rev={} lines {}-{} plain text (Vtok saves less than min-gain here)\n---\n{text}", fnv(src), lines.0, lines.1)
}

/// The plain lines of a symbol (`name`, `Class.method`), with its first and last line; None for chunk ids and unknown names.
pub(super) fn symbol_text(lang: &str, src: &str, id: &str) -> Option<(String, usize, usize)> {
    let (_, start, end) = symbols(lang, src).ok()?.into_iter().find(|(sid, _, _)| sid == id)?;
    let text = src.lines().skip(start.saturating_sub(1)).take(end + 1 - start).collect::<Vec<_>>().join("\n");
    Some((text, start, end))
}

/// Writes `out` over `path` through a temp file, with the line endings the file had, unless `dry_run`; says what happened.
pub(super) fn save(path: &str, src: &str, out: &str, crlf: bool, dry_run: bool) -> Result<String, String> {
    let delta = out.lines().count() as i64 - src.lines().count() as i64;
    if !dry_run {
        let text = if crlf { out.replace('\n', "\r\n") } else { out.to_string() };
        let tmp = format!("{path}.vtok.tmp");
        std::fs::write(&tmp, &text).map_err(|e| format!("cannot write {tmp}: {e}"))?;
        std::fs::rename(&tmp, path).map_err(|e| format!("cannot replace {path}: {e}"))?;
    }
    Ok(format!("{path}: {} ({delta:+} lines); new rev={}", if dry_run { "dry run, not written" } else { "written" }, fnv(out)))
}

/// `vtok_edit` with `replace: [{old, new, all?}]`: exact-text edits to any text file, checked against the rev. Each `old`
/// must occur once unless `all` is set, and nothing is written unless every replacement applies.
pub(super) fn replace(a: &Value) -> Result<String, String> {
    let (path, rev) = (arg_str(a, "path")?, arg_str(a, "rev")?);
    let (src, crlf) = read_text(path)?;
    if fnv(&src) != rev {
        return Err(format!("{path} changed since it was read (rev {rev} != {}); read it again before editing", fnv(&src)));
    }
    let list = a.get("replace").and_then(|v| v.as_array()).ok_or("\"replace\" must be a list of {old, new}")?;
    let mut out = src.clone();
    for (i, r) in list.iter().enumerate() {
        let text = |k: &str| r.get(k).and_then(|v| v.as_str()).map(normalize_newlines).ok_or(format!("replace[{i}]: missing string \"{k}\""));
        let (old, new) = (text("old")?, text("new")?);
        let all = r.get("all").and_then(|v| v.as_bool()).unwrap_or(false);
        let n = if old.is_empty() { 0 } else { out.matches(old.as_str()).count() };
        if n == 0 { return Err(format!("replace[{i}]: text not found in {path}; nothing was written")); }
        if n > 1 && !all { return Err(format!("replace[{i}]: text occurs {n} times in {path}; add the lines around it or set \"all\": true; nothing was written")); }
        out = out.replace(old.as_str(), &new);
    }
    if out == src { return Ok(format!("{path}: no change")); }
    save(path, &src, &out, crlf, a.get("dry_run").and_then(|v| v.as_bool()).unwrap_or(false))
}
