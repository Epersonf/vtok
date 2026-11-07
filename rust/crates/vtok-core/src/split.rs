//! Split big files into addressable chunks; edit one chunk at a time; splice back into the ORIGINAL file.
//!
//! `chunks` plans the chunks, `render` shows them (alias table, index, one chunk), `apply` splices edits back and
//! `patch` keeps the original text of every token an edit left alone.
mod apply;
mod chunks;
mod patch;
mod render;

pub use apply::apply_edits;
pub use chunks::{plan_chunks, plan_symbols, resolve_chunks, Chunk};
pub use render::{chunk_source, chunk_vtok, file_table, plan_project, render_index};
