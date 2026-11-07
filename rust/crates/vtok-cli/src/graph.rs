//! `vtok graph`: a persistent, scope-aware relation graph of a project (functions, methods, types, files, plus
//! documents: markdown, text, docx, PDF, images, video/audio) with communities, hubs, path and explain queries. Built
//! from the plugins' symbol and identifier data, stored in `.vtok/graph.json`, rebuilt when a file is newer.
//! Every edge has a relation kind (calls, uses_type, references, implements, extends, contains, imports, tests,
//! mentions) and a score: 1.0 is EXTRACTED (syntax and scope rules name exactly one definition), lower is INFERRED
//! (the resolution heuristic's confidence, 0.3-0.95).
use std::path::Path;

#[path = "graph_bind.rs"] mod bind;
#[path = "graph_comm.rs"] mod comm;
#[path = "graph_docs.rs"] mod docs;
#[path = "graph_edges.rs"] mod edges;
#[path = "graph_explain.rs"] mod explain;
#[path = "graph_media.rs"] mod media;
#[path = "graph_names.rs"] mod names;
#[path = "graph_occ.rs"] mod occ;
#[path = "graph_pdf.rs"] mod pdf;
#[path = "graph_query.rs"] mod query;
#[path = "graph_refs.rs"] mod refs;
#[path = "graph_rels.rs"] mod rels;
#[path = "graph_resolve.rs"] mod resolve;
#[path = "graph_scope.rs"] mod scope;
#[path = "graph_scope_crates.rs"] mod scope_crates;
#[path = "graph_scope_langs.rs"] mod scope_langs;
#[path = "graph_scope_rs.rs"] mod scope_rs;
#[path = "graph_store.rs"] mod store;
#[path = "graph_text.rs"] mod text;
#[cfg(test)] #[path = "graph_tests.rs"] mod tests;
#[cfg(test)] #[path = "graph_tests_docs.rs"] mod tests_docs;
#[cfg(test)] #[path = "graph_tests_langs.rs"] mod tests_langs;
#[cfg(test)] #[path = "graph_tests_rels.rs"] mod tests_rels;

pub use store::{run, text_for};
#[cfg(test)] use store::load_or_build;

pub const EXTRACTED: u8 = 0;
pub const INFERRED: u8 = 1;
pub fn tag(c: u8) -> &'static str { if c == EXTRACTED { "EXTRACTED" } else { "INFERRED" } }

pub const RELS: &[&str] = &["calls", "uses_type", "references", "implements", "extends", "contains", "imports", "tests", "mentions"];
pub const CALLS: u8 = 0;
pub const USES_TYPE: u8 = 1;
pub const REFERENCES: u8 = 2;
pub const IMPLEMENTS: u8 = 3;
pub const EXTENDS: u8 = 4;
pub const CONTAINS: u8 = 5;
pub const IMPORTS: u8 = 6;
pub const TESTS: u8 = 7;
pub const MENTIONS: u8 = 8;
/// Structural relations: left out of degrees, hubs and communities.
pub fn structural(r: u8) -> bool { r == CONTAINS || r == IMPORTS }

/// `a`/`b` are the line range for code and text; width/height for images, page for pdf pages, pages for pdfs,
/// duration seconds / size in KB for video and audio.
#[derive(Clone)]
pub struct Node { pub file: String, pub name: String, pub a: usize, pub b: usize, pub kind: &'static str }

pub const KINDS: &[&str] = &["function", "method", "type", "value", "file", "doc", "pdf", "page", "image", "video", "audio"];

impl Node {
    pub fn is_code(&self) -> bool { matches!(self.kind, "function" | "method" | "type" | "value") }
    /// Short facts for non-code nodes: dimensions, page count, duration and size.
    pub fn detail(&self) -> String {
        match self.kind {
            "image" => format!("{}x{} px", self.a, self.b),
            "pdf" => format!("{} pages", self.b),
            "page" => format!("page {}", self.a),
            "video" | "audio" => format!("{}{} KB", if self.a > 0 { format!("{}:{:02}, ", self.a / 60, self.a % 60) } else { String::new() }, self.b),
            _ => format!("{} lines", self.b),
        }
    }
    /// The type a method belongs to: `Graph.find` -> Graph, `Language for Rust.name` -> Rust.
    pub fn owner(&self) -> &str {
        let o = self.name.rsplit_once('.').map_or("", |x| x.0);
        let o = o.rsplit_once(" for ").map_or(o, |x| x.1);
        o.rsplit('.').next().unwrap_or(o)
    }
    /// The trait of a method defined in `impl Trait for Type`.
    pub fn impl_trait(&self) -> Option<&str> {
        let o = self.name.rsplit_once('.')?.0;
        o.split_once(" for ").map(|x| x.0.rsplit('.').next().unwrap_or(x.0))
    }
    /// Display name: the symbol's base name, or the file name for documents.
    pub fn label(&self) -> String {
        if self.is_code() { crate::refs::base(&self.name).to_string() } else { self.name.rsplit('/').next().unwrap_or(&self.name).to_string() }
    }
}

#[derive(Default)]
pub struct Graph {
    pub nodes: Vec<Node>,
    pub edges: Vec<(usize, usize)>,
    pub conf: Vec<u8>,
    /// relation (index into RELS) and score in percent (100 = EXTRACTED), parallel to `edges`
    pub rel: Vec<u8>,
    pub score: Vec<u8>,
    pub files: usize,
    pub label: Vec<usize>,
}

impl Graph {
    pub(crate) fn finish(&mut self, m: edges::EdgeMap) {
        self.edges = m.keys().map(|k| (k.0, k.1)).collect();
        self.rel = m.keys().map(|k| k.2).collect();
        self.score = m.values().copied().collect();
        self.conf = self.score.iter().map(|&s| if s >= 100 { EXTRACTED } else { INFERRED }).collect();
        self.label = comm::communities(&self.nodes, &self.semantic().collect::<Vec<_>>());
    }
    /// Edges that carry meaning (everything but contains/imports): degrees, hubs, communities.
    pub(crate) fn semantic(&self) -> impl Iterator<Item = (usize, usize)> + '_ {
        self.edges.iter().zip(&self.rel).filter(|(_, r)| !structural(**r)).map(|(e, _)| *e)
    }
}

pub fn build(root: &Path) -> Graph {
    let mut g = Graph::default();
    let parsed = edges::parse_files(root, &mut g);
    g.files = parsed.len();
    let by_base = edges::by_base(&g.nodes);
    let scopes = scope::build(root, &parsed);
    let mut m = resolve::resolve(&g.nodes, &parsed, &scopes, &by_base);
    rels::add(&g.nodes, &parsed, &scopes, &by_base, &mut m);
    docs::add(root, &mut g, &parsed, &by_base, &mut m);
    g.finish(m);
    g
}
