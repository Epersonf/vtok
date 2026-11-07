//! Python plugin: native-tight notation built from the tree-sitter CST.
//!
//! The text stays real Python (1-space indent, no optional whitespace, inline single-line bodies,
//! `;`-joined simple statements, comments and docstrings removed, aliased names). Decoding only
//! reverses aliases and hidden literals, so the model's output is valid Python as written.
//!
//! `tree` (parsing, node helpers) feeds `ctx` and `emit` (nodes to notation), `analysis` (name census) and `canon`
//! (tree dump for verification); `codec` is `encode`/`decode`, with `docs` and `pretty` laying decoded code out again;
//! `plugin` is the `Language` implementation (`plugin/` holds its units, token spans and chunk codec).
mod analysis;
mod canon;
mod codec;
mod ctx;
mod docs;
mod emit;
mod plugin;
mod pretty;
mod tree;

pub use analysis::{analyze, Analysis};
pub use codec::{decode, decode_raw, encode};
pub use docs::{collect_docs, Docs};
pub use plugin::Python;
pub use pretty::pretty;
pub use tree::{first_error, parse};
pub use vtok_core::lang::{EncodeOpts, Encoded, View};

static PY_NAMES: &str = include_str!("../../vtok-core/data_py_names.txt");
const IMPORTS: [&str; 3] = ["import_statement", "import_from_statement", "future_import_statement"];
const CLAUSES: [&str; 6] = ["elif_clause", "else_clause", "except_clause", "except_group_clause", "finally_clause", "case_clause"];
