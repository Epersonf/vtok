//! Rust plugin: native minified notation (tree-sitter). Every token is kept (`;` and `,` included), only
//! comments and optional whitespace go away; identifiers are aliased through the table.
//!
//! `tree` (parsing, node helpers) feeds `ctx` and `flat` (nodes to notation), `analysis` (name census), `literals`
//! (hidden string literals) and `canon` (tree dump for verification); `codec` is `encode`/`decode`; `idents` finds
//! identifier spans; `plugin` is the `Language` implementation (`plugin/` holds its units, spans and chunk codec).
mod analysis;
mod canon;
mod codec;
mod ctx;
mod flat;
mod idents;
mod literals;
mod plugin;
mod tree;

pub use analysis::{analyze, Analysis};
pub use codec::{decode, encode};
pub use plugin::Rust;
pub use tree::{first_error, parse};
pub use vtok_core::lang::{EncodeOpts, Encoded, View};

const IDENT: [&str; 4] = ["identifier", "type_identifier", "field_identifier", "shorthand_field_identifier"];
const COMMENTS: [&str; 2] = ["line_comment", "block_comment"];
const SKIP: [&str; 4] = ["line_comment", "block_comment", "empty_statement", "shebang"];
const STRINGS: [&str; 2] = ["string_literal", "raw_string_literal"];
const LEAF_TEXT: [&str; 3] = ["string_literal", "raw_string_literal", "char_literal"];
