//! Tools that read or change one source file: read, chunk, outline, edit, stats.
use super::args::{arg_str, fnv, max_lines, read_source};
use super::notation;
use super::plain;
use serde_json::Value;
use std::collections::BTreeMap;
use vtok_api::*;

pub(super) fn tool_read(a: &Value) -> std::result::Result<String, String> {
    let path = arg_str(a, "path")?;
    if !plain::is_source(path) { return plain::read(path); }
    let (src, lang, _) = read_source(path)?;
    let view = a.get("view").and_then(|v| v.as_str()).unwrap_or("full");
    let rev = fnv(&src);
    let raw_tokens = ntok(&src);
    let (body, table, note) = if view == "full" {
        let s = split_file(lang.name(), &src, max_lines(a)).map_err(|e| e.to_string())?;
        let n = if s.whole_file { format!("1 chunk (c1 = whole file), {} lines", src.lines().count()) } else { format!("{} chunks, {} lines", s.chunks.len(), src.lines().count()) };
        (s.index, s.table, n)
    } else {
        let o = Options { view: Some(view.to_string()), ..Default::default() };
        let r = encode(lang.name(), &src, &o).map_err(|e| e.to_string())?;
        (r.vtok, r.table.clone(), format!("{view} view (read-only)"))
    };
    let tbl = render_table(&table);
    let total = ntok(&body) + ntok(&tbl);
    let compact = format!("# {path} lang={} rev={rev} {note}; {raw_tokens} raw tokens -> {total}\n# aliases (alias=original)\n{tbl}\n---\n{body}", lang.name());
    if view == "full" {
        let plain_out = plain::plain_read(path, &src);
        if !plain::pays(&compact, &plain_out, plain::gain_near(path)) {
            crate::usage::record("read", raw_tokens, ntok(&plain_out), path);
            return Ok(plain_out);
        }
    }
    crate::usage::record("read", raw_tokens, total, path);
    Ok(format!("{}{compact}", notation::first_use(lang.name())))
}

pub(super) fn tool_chunk(a: &Value) -> std::result::Result<String, String> {
    let path = arg_str(a, "path")?;
    let id = arg_str(a, "id")?;
    let (src, lang, _) = read_source(path)?;
    let (v, used) = chunk_with_aliases(lang.name(), &src, id, max_lines(a)).map_err(|e| e.to_string())?;
    let aliases = if used.is_empty() { String::new() } else { format!("# aliases used (alias=original)\n{}\n", render_table(&used)) };
    let out = format!("# {path} chunk {id} rev={}\n{aliases}---\n{v}", fnv(&src));
    if let Some((text, from, to)) = plain::symbol_text(lang.name(), &src, id) {
        let plain_out = plain::plain_chunk(path, id, &src, &text, (from, to));
        if !plain::pays(&out, &plain_out, plain::gain_near(path)) {
            crate::usage::record("chunk", ntok(&src), ntok(&plain_out), &format!("{path} {id}"));
            return Ok(plain_out);
        }
    }
    crate::usage::record("chunk", ntok(&src), ntok(&out), &format!("{path} {id}"));
    Ok(format!("{}{out}", notation::first_use(lang.name())))
}

pub(super) fn tool_outline(a: &Value) -> std::result::Result<String, String> {
    let path = arg_str(a, "path")?;
    let (src, lang, _) = read_source(path)?;
    let o = outline(lang.name(), &src).map_err(|e| e.to_string())?;
    let out = format!("# {path} lang={} rev={} {} lines, {} raw tokens\n---\n{o}", lang.name(), fnv(&src), src.lines().count(), ntok(&src));
    crate::usage::record("outline", ntok(&src), ntok(&out), path);
    Ok(out)
}

pub(super) fn tool_edit(a: &Value) -> std::result::Result<String, String> {
    if a.get("replace").is_some() { return plain::replace(a); }
    let path = arg_str(a, "path")?;
    let rev = arg_str(a, "rev")?;
    let (src, lang, crlf) = read_source(path)?;
    if fnv(&src) != rev {
        return Err(format!("{path} changed since it was read (rev {rev} != {}); read it again before editing", fnv(&src)));
    }
    let chunks = a.get("chunks").and_then(|v| v.as_object()).ok_or("missing \"chunks\" (or \"replace\")")?;
    let mut edits: BTreeMap<String, String> = BTreeMap::new();
    for (k, v) in chunks { edits.insert(k.clone(), v.as_str().ok_or("chunk values must be strings")?.to_string()); }
    let out = apply_chunks(lang.name(), &src, &edits, max_lines(a), false).map_err(|e| e.to_string())?;
    if out == src { return Ok(format!("{path}: no change (the edited chunks are equivalent to the original)")); }
    plain::save(path, &src, &out, crlf, a.get("dry_run").and_then(|v| v.as_bool()).unwrap_or(false))
}

pub(super) fn tool_stats(a: &Value) -> std::result::Result<String, String> {
    let paths = a.get("paths").and_then(|v| v.as_array()).ok_or("missing array argument \"paths\"")?;
    let (mut raw, mut vt) = (0usize, 0usize);
    let mut rows = vec![];
    for p in paths {
        let p = p.as_str().ok_or("paths must be strings")?;
        match read_source(p).and_then(|(src, lang, _)| encode(lang.name(), &src, &Options::default()).map(|r| (src, r)).map_err(|e| e.to_string())) {
            Ok((src, r)) => {
                let (a, b) = (ntok(&src), ntok(&r.vtok) + ntok(&render_table(&r.table)));
                raw += a; vt += b;
                rows.push(format!("{p}: {a} -> {b} ({:.0}%)", 100.0 * (1.0 - b as f64 / a.max(1) as f64)));
            }
            Err(e) => rows.push(format!("{p}: {e}")),
        }
    }
    rows.push(format!("total: {raw} -> {vt} tokens ({:.0}% saved)", 100.0 * (1.0 - vt as f64 / raw.max(1) as f64)));
    Ok(rows.join("\n"))
}
