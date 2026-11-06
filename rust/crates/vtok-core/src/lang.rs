//! Types shared by every language plugin.
use std::collections::BTreeMap;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum View {
    Full,
    Skeleton,
    Lean,
}

impl View {
    pub fn parse(s: &str) -> Option<View> {
        match s {
            "full" => Some(View::Full),
            "skeleton" => Some(View::Skeleton),
            "lean" => Some(View::Lean),
            _ => None,
        }
    }
}

pub struct Encoded {
    pub vtok: String,
    /// original -> alias
    pub table: BTreeMap<String, String>,
    /// placeholder -> hidden literal text (kept locally, never shown to the model)
    pub literals: BTreeMap<String, String>,
}

pub struct EncodeOpts {
    pub alias: bool,
    pub verify: bool,
    pub view: View,
    pub hide_literals: usize,
    pub table: Option<BTreeMap<String, String>>,
}

impl Default for EncodeOpts {
    fn default() -> Self {
        EncodeOpts { alias: true, verify: true, view: View::Full, hide_literals: 0, table: None }
    }
}
