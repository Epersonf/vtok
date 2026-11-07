//! Tools that look across a project: refs, find (optionally with the code of the best matches), map, graph.
use super::args::arg_str;
use super::source::tool_chunk;
use serde_json::{json, Value};
use vtok_api::ntok;

pub(super) fn tool_refs(a: &Value) -> std::result::Result<String, String> {
    let name = arg_str(a, "name")?;
    if name.is_empty() || !name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '$') { return Err("name must be a single identifier".into()); }
    let dir = a.get("path").and_then(|v| v.as_str()).unwrap_or(".");
    let budget = a.get("max_tokens").and_then(|v| v.as_u64()).map_or(500, |n| n.clamp(100, 8000) as usize);
    let r = crate::refs::find(name, std::path::Path::new(dir), budget);
    crate::usage::record("refs", r.raw_tokens.max(ntok(&r.text)), ntok(&r.text), name);
    Ok(r.text)
}

pub(super) fn tool_find(a: &Value) -> std::result::Result<String, String> {
    let q = arg_str(a, "query")?;
    let dir = a.get("path").and_then(|v| v.as_str()).unwrap_or(".");
    let budget = a.get("max_tokens").and_then(|v| v.as_u64()).map_or(400, |n| n.clamp(100, 8000) as usize);
    let show = a.get("show").and_then(|v| v.as_u64()).unwrap_or(0).min(3) as usize;
    let r = crate::refs::find_symbols(q, std::path::Path::new(dir), budget);
    let mut out = r.text.clone();
    // one call instead of find + chunk: the compact code of the best matches
    for (rel, id) in r.top.iter().take(show) {
        let full = std::path::Path::new(dir).join(rel);
        match tool_chunk(&json!({"path": full.to_string_lossy(), "id": id})) {
            Ok(c) if ntok(&c) <= 700 => { out.push('\n'); out.push_str(&c); }
            Ok(c) => out.push_str(&format!("\n# {rel} {id}: {} tokens, too long to inline; use vtok_chunk\n", ntok(&c))),
            Err(e) => out.push_str(&format!("\n# {rel} {id}: {e}\n")),
        }
    }
    crate::usage::record("find", r.raw_tokens.max(ntok(&out)), ntok(&out), q);
    Ok(out)
}

pub(super) fn tool_map(a: &Value) -> std::result::Result<String, String> {
    let dir = a.get("path").and_then(|v| v.as_str()).unwrap_or(".");
    let budget = a.get("max_tokens").and_then(|v| v.as_u64()).map_or(700, |n| n.clamp(200, 8000) as usize);
    let r = crate::refs::map(std::path::Path::new(dir), budget);
    crate::usage::record("map", r.raw_tokens.max(ntok(&r.text)), ntok(&r.text), dir);
    Ok(r.text)
}

pub(super) fn tool_graph(a: &Value) -> std::result::Result<String, String> {
    let dir = a.get("path").and_then(|v| v.as_str()).unwrap_or(".");
    let action = arg_str(a, "action")?;
    let t = crate::graph::text_for(action, a.get("a").and_then(|v| v.as_str()), a.get("b").and_then(|v| v.as_str()), std::path::Path::new(dir))?;
    crate::usage::record("graph", ntok(&t) * 4, ntok(&t), action);
    Ok(t)
}
