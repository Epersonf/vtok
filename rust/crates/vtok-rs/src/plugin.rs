//! `Language` implementation for Rust (chunking hooks included).
use super::*;
use vtok_core::language::{Analysis as CoreAnalysis, Language, Unit};

pub struct Rust;

const BODY_ITEMS: [&str; 3] = ["impl_item", "trait_item", "mod_item"];

static PROMPT: &str = "Vtok-Rust: minified Rust. One top-level item per line, no comments, no optional spaces.\nEvery ';' and ',' is kept. Valid Rust otherwise.\nNames in the alias table (alias=original) are used shortened; new names are written in full.\nExample:\npub fn val(u:&str,p:&str)->bool{if u.is_empty()||p.is_empty(){return false}db.q(u,p)}\n";

fn rng(n: Node) -> (usize, usize) { (n.start_position().row + 1, n.end_position().row + 1) }

fn base_type<'a>(n: Node<'a>, src: &[u8]) -> String {
    let n = if n.kind() == "generic_type" { n.child_by_field_name("type").unwrap_or(n) } else { n };
    text(n, src).to_string()
}

fn item_name(n: Node, src: &[u8]) -> String {
    match n.kind() {
        "impl_item" => {
            let ty = n.child_by_field_name("type").map(|t| base_type(t, src)).unwrap_or_default();
            match n.child_by_field_name("trait") {
                Some(tr) => format!("{} for {ty}", base_type(tr, src)),
                None => ty,
            }
        }
        "use_declaration" => "use".into(),
        "extern_crate_declaration" => "extern".into(),
        "attribute_item" | "inner_attribute_item" => "attr".into(),
        "expression_statement" | "macro_invocation" => {
            let m = if n.kind() == "macro_invocation" { Some(n) } else { named_children(n).into_iter().find(|c| c.kind() == "macro_invocation") };
            m.and_then(|m| m.child_by_field_name("macro")).map(|x| format!("{}!", text(x, src))).unwrap_or_else(|| n.kind().to_string())
        }
        k => n.child_by_field_name("name").map(|x| text(x, src).to_string()).unwrap_or_else(|| k.to_string()),
    }
}

fn wrapped(text: &str, member: bool) -> String {
    if member { format!("mod {WRAP} {{\n{text}\n}}") } else { text.to_string() }
}

fn wrapper_members<'a>(root: Node<'a>) -> Vec<Node<'a>> {
    named_children(root).into_iter().next()
        .and_then(|c| c.child_by_field_name("body"))
        .map(named_children).unwrap_or_default()
}

/// Items of `nodes` as units; attributes join the item that follows them.
fn push_units(nodes: &[Node], src: &[u8], max_lines: usize, parent: Option<&str>, out: &mut Vec<Unit>) {
    let mut pending: Option<Node> = None;
    for &n in nodes {
        if COMMENTS.contains(&n.kind()) { continue; }
        if matches!(n.kind(), "attribute_item") {
            if pending.is_none() { pending = Some(n); }
            continue;
        }
        let first = pending.take().unwrap_or(n);
        let (s, e) = (first.start_position().row + 1, n.end_position().row + 1);
        let member = parent.is_some();
        if !member && BODY_ITEMS.contains(&n.kind()) && e - s + 1 > max_lines {
            if let Some(body) = n.child_by_field_name("body") {
                let name = item_name(n, src);
                push_units(&named_children(body), src, max_lines, Some(&name), out);
                continue;
            }
        }
        out.push(Unit {
            name: item_name(n, src), kind: n.kind().into(), start: s, end: e, member,
            parent: parent.map(String::from), indent: first.start_position().column,
        });
    }
    if let Some(p) = pending {
        let (s, e) = rng(p);
        out.push(Unit { name: "attr".into(), kind: p.kind().into(), start: s, end: e, member: parent.is_some(), parent: parent.map(String::from), indent: p.start_position().column });
    }
}

impl Language for Rust {
    fn name(&self) -> &'static str { "rust" }
    fn extensions(&self) -> &'static [&'static str] { &[".rs"] }
    fn grammar_prompt(&self) -> &'static str { PROMPT }

    fn encode(&self, source: &str, opts: &EncodeOpts) -> Result<Encoded> { encode(source, opts) }

    fn decode(&self, vtok: &str, table: &HashMap<String, String>, literals: &HashMap<String, String>) -> Result<String> {
        decode(vtok, table, literals)
    }

    fn analyze(&self, source: &str) -> Result<CoreAnalysis> {
        let source = source.replace("\r\n", "\n");
        let tree = parse(source.as_bytes());
        let a = analyze(tree.root_node(), source.as_bytes());
        Ok(CoreAnalysis { counts: a.counts, excluded: a.excl, avoid: a.avoid, reserved: HashSet::new() })
    }

    fn units(&self, source: &str, max_lines: usize) -> Result<Vec<Unit>> {
        let source = source.replace("\r\n", "\n");
        let tree = parse(source.as_bytes());
        let mut out = vec![];
        push_units(&named_children(tree.root_node()), source.as_bytes(), max_lines, None, &mut out);
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
        let out = decode(&format!("mod {WRAP}{{\n{vtok}\n}}\n"), inv, &none)?;
        let tree = parse(out.as_bytes());
        let pad = " ".repeat(indent);
        let mut s = String::new();
        for m in wrapper_members(tree.root_node()) {
            if COMMENTS.contains(&m.kind()) { continue; }
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
        Some(nodes.into_iter().filter(|n| !SKIP.contains(&n.kind())).map(|n| (n.start_byte() - off, n.end_byte() - off)).collect())
    }

    fn tokens(&self, text: &str, member: bool) -> Option<Vec<(usize, usize)>> {
        let w = wrapped(text, member);
        let off = if member { w.len() - text.len() - 2 } else { 0 };
        let tree = parse(w.as_bytes());
        if first_error(tree.root_node()).is_some() { return None; }
        let mut out = vec![];
        let mut st = vec![tree.root_node()];
        while let Some(n) = st.pop() {
            if SKIP.contains(&n.kind()) { continue; }
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
            Some(e) => Err(VtokError::at(format!("result is not valid Rust near line {}", e.start_position().row + 1), e.start_position().row + 1)),
            None => Ok(()),
        }
    }
}
