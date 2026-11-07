//! Large data files (CSV, TSV, JSON, NDJSON, YAML) that repeat one record shape hundreds of times. A head/tail cut or
//! a first-record outline hides the one record that does not fit, so this view keeps what a reader needs from such a
//! file: how many records, how every field is distributed, the records that stand out (a value seen once, a number far
//! outside the rest, a missing or extra field) verbatim with their line numbers, a few sample rows and the last row.
//! The whole file stays in the stored output. Readers: `csv`, `json` (also newline-delimited), `yaml`; the shared
//! analysis is `stats` (field kinds) and `flags` (odd cells); `describe` and `render` write the text. Objects that share
//! no fields (a schema, a workflow, a lockfile) are not records and keep the older views; `rest` names what a document
//! holds besides its records.
mod csv;
mod describe;
mod flags;
mod flags_cols;
mod flags_num;
mod json;
mod records;
mod render;
mod rest;
mod stats;
mod yaml;
mod yaml_scalar;

#[cfg(test)] mod t_csv;
#[cfg(test)] mod t_edge;
#[cfg(test)] mod t_flags;
#[cfg(test)] mod t_json;
#[cfg(test)] mod t_wire;
#[cfg(test)] mod t_yaml;

/// Fewer records than this and the lines are shown as they are.
const MIN_ROWS: usize = 20;
/// Larger input is left to the generic summary (the analysis is linear, but 40 MB of records is not worth waiting for).
const MAX_BYTES: usize = 40 << 20;
/// Extensions whose content is plain text of unknown layout; any other extension names a format of its own.
const PLAIN: [&str; 8] = ["txt", "text", "dat", "data", "out", "tab", "list", "lst"];

/// The view of `lines`, or None when they are not a long list of similar records. `ext` is the extension of the file
/// the command reads, when it names one.
pub fn view(lines: &[String], ext: Option<&str>) -> Option<String> {
    // each reader asks for `MIN_ROWS` records itself; a minified JSON document may hold them all on one line
    let raw: usize = lines.iter().map(|l| l.len() + 1).sum();
    if raw > MAX_BYTES { return None; }
    let ext = ext.map(str::to_ascii_lowercase);
    let table = match ext.as_deref() {
        Some("csv" | "tsv" | "psv") => csv::read(lines, true),
        Some("json" | "geojson" | "har") => json::read(lines).or_else(|| json::read_lines(lines)),
        Some("ndjson" | "jsonl") => json::read_lines(lines),
        Some("yaml" | "yml") => yaml::read(lines),
        Some(e) if !PLAIN.contains(&e) => None,
        _ => sniff(lines),
    }?;
    let text = render::view(&table)?;
    (text.len() * 2 <= raw).then_some(text)
}

/// The view of what a command printed that no dedicated view claimed, recognised from the text alone. A command that
/// prints a named file is not asked: `views_sys` judged that file by its name, and a source file is not a table.
pub fn sniffed(cmd: &str, lines: &[String]) -> Option<String> {
    if crate::views_sys::prints_file(cmd) { None } else { view(lines, None) }
}

/// Output of any command: judged by its first character and its structure.
fn sniff(lines: &[String]) -> Option<records::Table> {
    let first = lines.iter().map(|l| l.trim_start()).find(|l| !l.is_empty())?;
    if first.starts_with(['{', '[']) { json::read(lines).or_else(|| json::read_lines(lines)) } else { csv::read(lines, false) }
}
