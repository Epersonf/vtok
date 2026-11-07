//! The minifier's exact-copy paths: whitespace that is data, unparsed regions, lexer-protected strings.
use crate::emit::Emit;
use crate::spec::Mode;
use crate::space::stringish;
use crate::tree::*;
use tree_sitter::Node;

impl<'a> Emit<'a> {
    /// Whitespace here is data (inside a string, a `<pre>`, raw script text): keep it as written.
    pub(crate) fn verbatim_context(&self, n: Node) -> bool {
        let mut p = n.parent();
        while let Some(a) = p {
            let k = a.kind();
            if stringish(k) && a.start_byte() < self.prev_end { return true; }
            if self.spec.mode == Mode::Html && matches!(k, "raw_text" | "script_element" | "style_element") && a.start_byte() < self.prev_end { return true; }
            if self.spec.mode == Mode::Html && k == "element" {
                let head = text(a, self.src).get(..10).unwrap_or("").to_ascii_lowercase();
                if (head.starts_with("<pre") || head.starts_with("<textarea")) && a.start_byte() < self.prev_end { return true; }
            }
            p = a.parent();
        }
        false
    }

    /// Source text between the previous token and `start`, with comments cut out.
    pub(crate) fn residual(&mut self, start: usize) -> String {
        let mut out = String::new();
        let mut at = self.prev_end;
        for &(s, e) in &self.skipped {
            if s >= at && s <= start { out.push_str(&String::from_utf8_lossy(&self.src[at..s])); at = e.min(start); }
        }
        if at < start { out.push_str(&String::from_utf8_lossy(&self.src[at..start])); }
        self.skipped.clear();
        out
    }

    /// Inside a string literal by the lexer's reading (only set for files with unparsed regions).
    pub(crate) fn protected(&self, at: usize) -> bool {
        let i = self.protect.partition_point(|s| s.0 <= at);
        i > 0 && at < self.protect[i - 1].1
    }

    /// `t` with the exact source gap before it (comments included).
    pub(crate) fn exact(&mut self, n: Node, t: String) {
        if !self.out.is_empty() { self.out.push_str(&String::from_utf8_lossy(&self.src[self.prev_end.min(n.start_byte())..n.start_byte()])); }
        self.skipped.clear();
        self.put(n, t);
    }

    /// An unparsed region: its exact text, with the exact source gap before it and after it.
    pub(crate) fn raw(&mut self, n: Node) {
        self.first.get_or_insert((n.start_byte(), true));
        self.exact(n, text(n, self.src).to_string());
        self.raw_next = true;
    }
}
