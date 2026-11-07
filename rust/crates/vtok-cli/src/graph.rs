//! `vtok graph`: a persistent, name-based call graph of a project (functions, methods, types, plus documents: markdown,
//! text, docx, PDF, images, video/audio) with communities, hubs, path and explain queries. Built from the same symbol
//! and identifier data as `refs`, stored in `.vtok/graph.json`, and rebuilt automatically when a file is newer.
//! Every edge carries a confidence tag: EXTRACTED (one candidate) or INFERRED (narrowed from several homonyms).
use std::path::Path;

#[path = "graph_comm.rs"] mod comm;
#[path = "graph_docs.rs"] mod docs;
#[path = "graph_edges.rs"] mod edges;
#[path = "graph_media.rs"] mod media;
#[path = "graph_pdf.rs"] mod pdf;
#[path = "graph_query.rs"] mod query;
#[path = "graph_refs.rs"] mod refs;
#[path = "graph_store.rs"] mod store;
#[path = "graph_text.rs"] mod text;
#[cfg(test)] #[path = "graph_tests.rs"] mod tests;
#[cfg(test)] #[path = "graph_tests_docs.rs"] mod tests_docs;

pub use store::{run, text_for};
#[cfg(test)] use store::load_or_build;

pub const EXTRACTED: u8 = 0;
pub const INFERRED: u8 = 1;
pub fn tag(c: u8) -> &'static str { if c == EXTRACTED { "EXTRACTED" } else { "INFERRED" } }

/// `a`/`b` are the line range for code and text; width/height for images, page for pdf pages, pages for pdfs,
/// duration seconds / size in KB for video and audio.
#[derive(Clone)]
pub struct Node { pub file: String, pub name: String, pub a: usize, pub b: usize, pub kind: &'static str }

pub const KINDS: &[&str] = &["function", "method", "type", "value", "doc", "pdf", "page", "image", "video", "audio"];

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
    /// Display name: the symbol's base name, or the file name for documents.
    pub fn label(&self) -> String {
        if self.is_code() { crate::refs::base(&self.name).to_string() } else { self.name.rsplit('/').next().unwrap_or(&self.name).to_string() }
    }
}

#[derive(Default)]
pub struct Graph { pub nodes: Vec<Node>, pub edges: Vec<(usize, usize)>, pub conf: Vec<u8>, pub files: usize, pub label: Vec<usize> }

impl Graph {
    pub(crate) fn finish(&mut self, m: edges::EdgeMap) {
        self.edges = m.keys().copied().collect();
        self.conf = m.values().copied().collect();
        self.label = comm::communities(&self.nodes, &self.edges);
    }
}

pub fn build(root: &Path) -> Graph {
    let mut g = Graph::default();
    let parsed = edges::parse_files(root, &mut g);
    g.files = parsed.len();
    let by_base = edges::by_base(&g.nodes);
    let mut m = edges::resolve(&g.nodes, &parsed, &by_base);
    docs::add(root, &mut g, &parsed, &by_base, &mut m);
    g.finish(m);
    g
}
