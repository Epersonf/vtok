//! What the model reads: the alias table (per file or per project), the skeleton index with its chunk map, one chunk.
use super::chunks::Chunk;
use crate::aliasing::plan_aliases;
use crate::lang::{EncodeOpts, View};
use crate::language::{Analysis, Language};
use crate::Result;
use std::collections::BTreeMap;

/// One stable alias table for many files, paid once.
pub fn plan_project(lang: &dyn Language, sources: &[&str], min_count: usize) -> BTreeMap<String, String> {
    let mut total = Analysis::default();
    for s in sources {
        if let Ok(a) = lang.analyze(s) {
            for (k, v) in a.counts { *total.counts.entry(k).or_insert(0) += v; }
            total.excluded.extend(a.excluded);
            total.avoid.extend(a.avoid);
            total.reserved.extend(a.reserved);
        }
    }
    plan_aliases(&total.counts, &total.avoid, &total.reserved, &total.excluded, min_count)
}

pub fn file_table(lang: &dyn Language, source: &str) -> BTreeMap<String, String> {
    plan_project(lang, &[source], 2)
}

pub fn chunk_source(source: &str, c: &Chunk) -> String {
    source.split_inclusive('\n').skip(c.start - 1).take(c.end - c.start + 1).collect()
}

/// Skeleton Vtok + chunk map: what the model reads first.
pub fn render_index(lang: &dyn Language, source: &str, chunks: &[Chunk], table: &BTreeMap<String, String>) -> Result<String> {
    let opts = EncodeOpts { view: View::Skeleton, table: Some(table.clone()), ..Default::default() };
    let mut sk = lang.encode(source, &opts)?.vtok;
    let mut rows = vec![];
    for c in chunks {
        let lab = c.label.replace(':', ": ").split_whitespace().map(|w| table.get(w).cloned().unwrap_or_else(|| w.to_string())).collect::<Vec<_>>().join(" ");
        rows.push(format!("{} L{}-{} {}", c.id, c.start, c.end, lab));
    }
    sk.push_str("\n# chunks (fetch one by id)\n");
    sk.push_str(&rows.join("\n"));
    sk.push('\n');
    Ok(sk)
}

pub fn chunk_vtok(lang: &dyn Language, source: &str, c: &Chunk, table: &BTreeMap<String, String>) -> Result<String> {
    lang.encode_chunk(&chunk_source(source, c), table, c.member)
}
