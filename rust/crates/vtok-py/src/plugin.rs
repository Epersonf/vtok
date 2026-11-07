//! `Language` implementation for Python (chunking hooks included).
use super::*;
use vtok_core::language::{Analysis as CoreAnalysis, Language, Unit};

pub struct Python;

const WRAP: &str = "__vtok_wrap__";

static PROMPT: &str = "Vtok: Python with minimal whitespace. Valid Python otherwise: 1-space indent, no spaces around\noperators, a header may take one simple statement inline (`if x:return 1`), simple statements joined by ';'.\nNames in the alias table (alias=original) are used shortened; new names are written in full.\nExample:\nclass AuthMgr:\n def val(u,p)->bool:\n  if not u or not p:return False\n  return db.q(u,p)\n";

fn wrapped(text: &str, member: bool) -> String {
    if member { format!("class {WRAP}:\n{text}") } else { text.to_string() }
}

/// Statements of the module, or of the synthetic wrapper class when `member`.
fn body_of<'a>(root: Node<'a>, member: bool) -> Vec<Node<'a>> {
    if member {
        named_children(root).into_iter().find(|c| !trivia(*c))
            .and_then(|c| c.child_by_field_name("body"))
            .map(|b| named_children(b).into_iter().filter(|x| !trivia(*x)).collect())
            .unwrap_or_default()
    } else {
        named_children(root).into_iter().filter(|x| !trivia(*x)).collect()
    }
}

fn unit_label(n: Node, src: &[u8]) -> String {
    let d = def_of(n);
    if let Some(d) = d { return name_of(d, src); }
    match n.kind() {
        "import_statement" | "import_from_statement" | "future_import_statement" => "imports".into(),
        "expression_statement" => {
            let kids = named_children(n);
            if let Some(a) = kids.first() {
                if matches!(a.kind(), "assignment" | "augmented_assignment") {
                    let mut names = vec![];
                    let mut cur = *a;
                    loop {
                        if let Some(l) = cur.child_by_field_name("left") {
                            if l.kind() == "identifier" { names.push(text(l, src).to_string()); }
                        }
                        match cur.child_by_field_name("right") {
                            Some(r) if r.kind() == "assignment" => cur = r,
                            _ => break,
                        }
                    }
                    return if names.is_empty() { "assign".into() } else { names.join(",") };
                }
            }
            "code".into()
        }
        _ => "code".into(),
    }
}

fn unit_kind(n: Node) -> String { n.kind().to_string() }

impl Language for Python {
    fn name(&self) -> &'static str { "python" }
    fn extensions(&self) -> &'static [&'static str] { &[".py", ".pyi"] }
    fn grammar_prompt(&self) -> &'static str { PROMPT }

    fn encode(&self, source: &str, opts: &EncodeOpts) -> Result<Encoded> { encode(source, opts) }

    fn decode(&self, vtok: &str, table: &HashMap<String, String>, literals: &HashMap<String, String>) -> Result<String> {
        decode(vtok, table, literals)
    }

    fn analyze(&self, source: &str) -> Result<CoreAnalysis> {
        let source = source.replace("\r\n", "\n");
        let tree = parse(source.as_bytes());
        if let Some(e) = first_error(tree.root_node()) {
            return Err(VtokError::at("source has syntax errors", e.start_position().row + 1));
        }
        let a = analyze(tree.root_node(), source.as_bytes());
        Ok(CoreAnalysis { counts: a.counts, excluded: a.excluded, avoid: a.avoid, reserved: HashSet::new() })
    }

    fn tokens(&self, text: &str, member: bool) -> Option<Vec<(usize, usize)>> {
        let w = wrapped(text, member);
        let off = w.len() - text.len();
        let tree = parse(w.as_bytes());
        if first_error(tree.root_node()).is_some() { return None; }
        let mut out = vec![];
        let mut st = vec![tree.root_node()];
        while let Some(n) = st.pop() {
            if n.kind() == "comment" { continue; }
            if n.child_count() == 0 || n.kind() == "string" {
                let (a, b) = (n.start_byte(), n.end_byte());
                if b > a && a >= off && b - off <= text.len() && !w[a..b].trim().is_empty() { out.push((a - off, b - off)); }
            } else {
                let mut c = n.walk();
                let kids: Vec<_> = n.children(&mut c).collect();
                for k in kids.into_iter().rev() { st.push(k); }
            }
        }
        Some(out)
    }

    fn identifier_spans(&self, text: &str) -> Option<Vec<(usize, usize)>> {
        let tree = parse(text.as_bytes());
        if first_error(tree.root_node()).is_some() { return None; }
        let mut out = vec![];
        let mut st = vec![tree.root_node()];
        while let Some(n) = st.pop() {
            match n.kind() {
                "comment" | "string" => continue,
                "identifier" => { out.push((n.start_byte(), n.end_byte())); continue; }
                _ => {}
            }
            let mut c = n.walk();
            let kids: Vec<_> = n.children(&mut c).collect();
            for k in kids { st.push(k); }
        }
        out.sort();
        Some(out)
    }

    fn units(&self, source: &str, max_lines: usize) -> Result<Vec<Unit>> {
        let source = source.replace("\r\n", "\n");
        let src = source.as_bytes();
        let tree = parse(src);
        let mut out = vec![];
        for n in body_of(tree.root_node(), false) {
            let (s, e) = (n.start_position().row + 1, n.end_position().row + 1);
            let class = def_of(n).filter(|d| d.kind() == "class_definition");
            if let (Some(c), true) = (class, e - s + 1 > max_lines) {
                let cname = name_of(c, src);
                let body = c.child_by_field_name("body").map(|b| named_children(b).into_iter().filter(|x| !trivia(*x)).collect::<Vec<_>>()).unwrap_or_default();
                for (i, m) in body.iter().enumerate() {
                    if i == 0 && is_docstring(*m, src) { continue; } // class docstring stays in the shell
                    out.push(Unit {
                        name: unit_label(*m, src), kind: unit_kind(*m),
                        start: m.start_position().row + 1, end: m.end_position().row + 1,
                        member: true, parent: Some(cname.clone()), indent: m.start_position().column,
                    });
                }
            } else {
                out.push(Unit { name: unit_label(n, src), kind: unit_kind(n), start: s, end: e, member: false, parent: None, indent: n.start_position().column });
            }
        }
        Ok(out)
    }

    fn encode_chunk(&self, text: &str, table: &BTreeMap<String, String>, member: bool) -> Result<String> {
        let opts = EncodeOpts { table: Some(table.clone()), ..Default::default() };
        let vt = if !member {
            encode(text, &opts)?.vtok
        } else {
            let w = wrapped(text, true);
            let src = w.as_bytes();
            let tree = parse(src);
            if let Some(e) = first_error(tree.root_node()) {
                return Err(VtokError::at("source has syntax errors", e.start_position().row));
            }
            let empty: HashSet<String> = HashSet::new();
            let ctx = Ctx { ren: table, lean: false, skeleton: false, hide: 0, existing: &empty, lits: RefCell::new((BTreeMap::new(), HashMap::new(), 0)), src };
            let mut lines: Vec<Line> = vec![];
            for s in body_of(tree.root_node(), true) { emit_stmt(s, &ctx, 0, &mut lines); }
            render(lines)
        };
        // self-check: the chunk must decode back to something equivalent
        let inv: HashMap<String, String> = table.iter().map(|(o, a)| (a.clone(), o.clone())).collect();
        let pad = text.lines().find(|l| !l.trim().is_empty()).map(|l| l.len() - l.trim_start().len()).unwrap_or(0);
        let back = self.decode_chunk(&vt, &inv, member, if member { pad } else { 0 }, None)?;
        if !self.equivalent(text, &back, member) {
            return Err(VtokError::new("chunk roundtrip verification failed"));
        }
        Ok(vt)
    }

    fn decode_chunk(&self, vtok: &str, inv: &HashMap<String, String>, member: bool, indent: usize, original: Option<&str>) -> Result<String> {
        let raw = decode_raw(vtok, inv, &HashMap::new())?;
        let docs = match original {
            Some(o) => {
                let w = wrapped(o, member);
                let t = parse(w.as_bytes());
                if first_error(t.root_node()).is_some() { Docs::default() } else { collect_docs(&body_of(t.root_node(), member), w.as_bytes()) }
            }
            None => Docs::default(),
        };
        Ok(pretty(&raw, &docs, if member { indent } else { 0 }))
    }

    fn equivalent(&self, a: &str, b: &str, member: bool) -> bool {
        let (wa, wb) = (wrapped(a, member), wrapped(b, member));
        let (ta, tb) = (parse(wa.as_bytes()), parse(wb.as_bytes()));
        if first_error(ta.root_node()).is_some() || first_error(tb.root_node()).is_some() { return false; }
        let (mut x, mut y) = (vec![], vec![]);
        canon(ta.root_node(), wa.as_bytes(), true, &mut x);
        canon(tb.root_node(), wb.as_bytes(), true, &mut y);
        x == y
    }

    fn validate(&self, source: &str) -> Result<()> {
        let tree = parse(source.as_bytes());
        match first_error(tree.root_node()) {
            Some(e) => Err(VtokError::at("result is not valid Python", e.start_position().row + 1)),
            None => Ok(()),
        }
    }
}
