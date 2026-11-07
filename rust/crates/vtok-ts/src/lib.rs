//! TypeScript plugin: native minified notation (port of the Python reference implementation).
//!
//! `tree` (parsing, node helpers) feeds `ctx`, `flat` and `lines` (nodes to notation), `analysis` (name census),
//! `literals` (hidden string literals) and `canon` (tree dump for verification); `codec` is `encode`/`decode`;
//! `plugin` is the `Language` implementation (`plugin/` holds its units, item spans and chunk codec).
mod analysis;
mod canon;
mod codec;
mod ctx;
mod flat;
mod lines;
mod literals;
mod plugin;
mod tree;

pub use analysis::{analyze, Analysis};
pub use codec::{decode, encode};
pub use plugin::TypeScript;
pub use tree::{first_error, parse};
pub use vtok_core::lang::{EncodeOpts, Encoded, View};

pub const IDENT: [&str; 3] = ["identifier", "type_identifier", "property_identifier"];
const SKIP: [&str; 2] = ["comment", "empty_statement"];
const RESERVED_WORDS: &str = "f C I k l v m r R B K t x y S L w E e Y X";

fn is_ident(k: &str) -> bool { IDENT.contains(&k) }
