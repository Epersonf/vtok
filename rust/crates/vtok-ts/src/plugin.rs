//! `Language` implementation for TypeScript (chunking hooks included).
use super::*;
use vtok_core::language::{Analysis as CoreAnalysis, Language, Unit};

pub struct TypeScript;

const WRAP: &str = "__vtok_wrap__";
const CLASS_DECLS: [&str; 2] = ["class_declaration", "abstract_class_declaration"];

static PROMPT: &str = "Vtok-TS: minified TypeScript. One top-level statement per line, no comments, no optional spaces,\nno ';' before '}' or at line end. Valid TypeScript otherwise (braces kept).\nNames in the alias table (alias=original) are used shortened; new names are written in full.\nExample:\nexport function val(u:string,p:string):boolean{if(!u||!p)return false;for(const x of xs)use(x);return db.q(u,p)}\n";

fn node_name(n: Node, src: &[u8]) -> String {
    let d = if n.kind() == "export_statement" { n.child_by_field_name("declaration").unwrap_or(n) } else { n };
    if let Some(nm) = d.child_by_field_name("name") { return text(nm, src).to_string(); }
    if matches!(d.kind(), "lexical_declaration" | "variable_declaration") {
        let names: Vec<String> = named_children(d).into_iter()
            .filter(|vd| vd.kind() == "variable_declarator")
            .filter_map(|vd| vd.child_by_field_name("name").map(|x| text(x, src).to_string()))
            .collect();
        if !names.is_empty() { return names.join(","); }
    }
    if d.kind() == "import_statement" { return "imports".into(); }
    d.kind().to_string()
}

fn rng(n: Node) -> (usize, usize) { (n.start_position().row + 1, n.end_position().row + 1) }

fn wrapped(text: &str, member: bool) -> String {
    if member { format!("class {WRAP} {{\n{text}\n}}") } else { text.to_string() }
}

fn wrapper_members<'a>(root: Node<'a>) -> Vec<Node<'a>> {
    named_children(root).into_iter().next()
        .and_then(|c| c.child_by_field_name("body"))
        .map(named_children).unwrap_or_default()
}

impl Language for TypeScript {
    fn name(&self) -> &'static str { "typescript" }
    fn extensions(&self) -> &'static [&'static str] { &[".ts", ".mts", ".cts", ".js", ".mjs", ".cjs", ".tsx", ".jsx"] }
    fn grammar_prompt(&self) -> &'static str { PROMPT }

    fn encode(&self, source: &str, opts: &EncodeOpts) -> Result<Encoded> { encode(source, opts) }

    fn decode(&self, vtok: &str, table: &HashMap<String, String>, literals: &HashMap<String, String>) -> Result<String> {
        decode(vtok, table, literals)
    }

    fn analyze(&self, source: &str) -> Result<CoreAnalysis> {
        let source = source.replace("\r\n", "\n");
        let tree = parse(source.as_bytes());
        let a = analyze(tree.root_node(), source.as_bytes());
        Ok(CoreAnalysis { counts: a.counts, excluded: a.excl, avoid: a.avoid, reserved: words(RESERVED_WORDS).collect() })
    }

    fn units(&self, source: &str, max_lines: usize) -> Result<Vec<Unit>> {
        let source = source.replace("\r\n", "\n");
        let src = source.as_bytes();
        let tree = parse(src);
        let mut out = vec![];
        for n in named_children(tree.root_node()) {
            if n.kind() == "comment" { continue; }
            let (s, e) = rng(n);
            let d = if n.kind() == "export_statement" { n.child_by_field_name("declaration") } else { Some(n) };
            if let Some(d) = d {
                if CLASS_DECLS.contains(&d.kind()) && e - s + 1 > max_lines {
                    if let Some(body) = d.child_by_field_name("body") {
                        let cname = node_name(n, src);
                        for m in named_children(body) {
                            if m.kind() == "comment" { continue; }
                            let (ms, me) = rng(m);
                            let name = if m.child_by_field_name("name").is_some() { node_name(m, src) } else { m.kind().to_string() };
                            out.push(Unit { name, kind: m.kind().into(), start: ms, end: me, member: true, parent: Some(cname.clone()), indent: m.start_position().column });
                        }
                        continue;
                    }
                }
            }
            out.push(Unit { name: node_name(n, src), kind: n.kind().into(), start: s, end: e, member: false, parent: None, indent: n.start_position().column });
        }
        Ok(out)
    }

    fn encode_chunk(&self, text: &str, table: &BTreeMap<String, String>, member: bool) -> Result<String> {
        let opts = EncodeOpts { table: Some(table.clone()), ..Default::default() };
        if !member { return Ok(encode(text, &opts)?.vtok); }
        let w = wrapped(text, true);
        let src = w.as_bytes();
        let tree = parse(src);
        if let Some(e) = first_error(tree.root_node()) {
            return Err(VtokError::new(format!("source has syntax errors near line {}", e.start_position().row)));
        }
        let ctx = Ctx { ren: table, skeleton: false, lean: false, lits: &HashMap::new(), src };
        let members = wrapper_members(tree.root_node());
        Ok(native_lines(&members, &ctx)?.join("\n") + "\n")
    }

    fn decode_chunk(&self, vtok: &str, inv: &HashMap<String, String>, member: bool, indent: usize, _original: Option<&str>) -> Result<String> {
        let none = HashMap::new();
        if !member { return decode(vtok, inv, &none); }
        let out = decode(&format!("class {WRAP}{{\n{vtok}\n}}\n"), inv, &none)?;
        let tree = parse(out.as_bytes());
        let pad = " ".repeat(indent);
        let mut s = String::new();
        for m in wrapper_members(tree.root_node()) {
            if m.kind() == "comment" { continue; }
            s.push_str(&pad);
            s.push_str(text(m, out.as_bytes()));
            s.push('\n');
        }
        Ok(s)
    }

    fn item_spans(&self, text: &str, member: bool) -> Option<Vec<(usize, usize)>> {
        let w = wrapped(text, member);
        let off = if member { w.len() - text.len() - 2 } else { 0 };
        let tree = parse(w.as_bytes());
        if first_error(tree.root_node()).is_some() { return None; }
        let nodes = if member { wrapper_members(tree.root_node()) } else { named_children(tree.root_node()) };
        Some(nodes.into_iter().filter(|n| !(n.kind() == "comment")).map(|n| (n.start_byte() - off, n.end_byte() - off)).collect())
    }

    fn tokens(&self, text: &str, member: bool) -> Option<Vec<(usize, usize)>> {
        let w = wrapped(text, member);
        let off = if member { w.len() - text.len() - 2 } else { 0 };
        let tree = parse(w.as_bytes());
        if first_error(tree.root_node()).is_some() { return None; }
        let mut out = vec![];
        let mut st = vec![tree.root_node()];
        while let Some(n) = st.pop() {
            if n.kind() == "comment" { continue; }
            if n.child_count() == 0 {
                if n.end_byte() > n.start_byte() && n.start_byte() >= off && n.end_byte() - off <= text.len() { out.push((n.start_byte() - off, n.end_byte() - off)); }
            } else {
                let mut c = n.walk();
                let kids: Vec<_> = n.children(&mut c).collect();
                for k in kids.into_iter().rev() { st.push(k); }
            }
        }
        Some(out)
    }

    fn equivalent(&self, a: &str, b: &str, member: bool) -> bool {
        let (wa, wb) = (wrapped(a, member), wrapped(b, member));
        let (ta, tb) = (parse(wa.as_bytes()), parse(wb.as_bytes()));
        if first_error(ta.root_node()).is_some() || first_error(tb.root_node()).is_some() { return false; }
        let (mut x, mut y) = (vec![], vec![]);
        canon(ta.root_node(), wa.as_bytes(), &mut x);
        canon(tb.root_node(), wb.as_bytes(), &mut y);
        x == y
    }

    fn validate(&self, source: &str) -> Result<()> {
        let tree = parse(source.as_bytes());
        match first_error(tree.root_node()) {
            Some(e) => Err(VtokError::at(format!("result is not valid TypeScript near line {}", e.start_position().row + 1), e.start_position().row + 1)),
            None => Ok(()),
        }
    }
}
