//! Encode options as a caller states them (all optional), and the string-based encode / decode entry points.
use crate::{language, EncodeOpts, Encoded, Result, View, VtokError};
use std::collections::{BTreeMap, HashMap};

#[derive(Default, Clone)]
pub struct Options {
    pub alias: Option<bool>,
    pub view: Option<String>,
    pub hide_literals: Option<usize>,
    pub table: Option<BTreeMap<String, String>>,
    pub verify: Option<bool>,
}

impl Options {
    pub fn to_opts(&self) -> Result<EncodeOpts> {
        let mut o = EncodeOpts::default();
        if let Some(a) = self.alias { o.alias = a; }
        if let Some(v) = &self.view { o.view = View::parse(v).ok_or_else(|| VtokError::new(format!("unknown view {v:?} (full|skeleton|lean)")))?; }
        if let Some(h) = self.hide_literals { o.hide_literals = h; }
        if let Some(v) = self.verify { o.verify = v; }
        o.table = self.table.clone();
        Ok(o)
    }
}

pub fn encode(lang: &str, source: &str, o: &Options) -> Result<Encoded> {
    language(lang)?.encode(source, &o.to_opts()?)
}

pub fn decode(lang: &str, vtok: &str, table: &HashMap<String, String>, literals: &HashMap<String, String>) -> Result<String> {
    language(lang)?.decode(vtok, table, literals)
}
