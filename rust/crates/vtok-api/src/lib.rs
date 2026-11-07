//! High-level, string-based API shared by the CLI and the pip / npm bindings.
//!
//! `registry` finds a language plugin, `options` and `chunks` are the encode / split / edit entry points and `idents`
//! lists identifier occurrences; everything is re-exported here.
pub use vtok_core::lang::{EncodeOpts, Encoded, View};
pub use vtok_core::language::Language;
pub use vtok_core::split::{self, Chunk};
pub use vtok_core::{parse_table, render_table, tokens::ntok, Result, VtokError};

mod chunks;
mod idents;
mod options;
mod registry;

pub use chunks::{apply_chunks, chunk_of, chunk_with_aliases, outline, project_table, split_file, symbols, SplitResult};
pub use idents::{idents, idents_dot};
pub use options::{decode, encode, Options};
pub use registry::{language, language_for_path, languages, normalize_newlines};
