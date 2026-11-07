//! Identifier occurrences of a source file, classified by syntax position: a bare name, a member (`x.f`, with the
//! receiver when it is a plain name), or a path (`a::b::f`, with the qualifier segments); whether it is called, a
//! macro, a definition name, a field key, or a path head itself. Language-agnostic: works on the plugin's identifier
//! spans plus the surrounding text.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Form { Bare, Member, Path }

#[derive(Debug)]
pub(crate) struct Occ {
    pub name: String,
    pub line: usize,
    pub form: Form,
    /// `x` in `x.f`; `None` when the receiver is an expression (`a().f`, `v[0].f`) or for other forms
    pub recv: Option<String>,
    /// `["crate", "refs"]` in `crate::refs::f`
    pub qual: Vec<String>,
    pub call: bool,
    /// followed by `::` (it qualifies the next name: `Type::new`, `module::f`)
    pub head: bool,
}

const DEF_KW: &[&str] = &["fn", "struct", "enum", "trait", "type", "union", "mod", "const", "static", "def", "class", "function", "interface", "func", "macro_rules", "let", "var", "val", "namespace", "package", "use", "import"];

fn ident_before(h: &str) -> Option<&str> {
    let t = h.trim_end();
    let start = t.char_indices().rev().take_while(|(_, c)| c.is_alphanumeric() || *c == '_' || *c == '$').last().map(|(i, _)| i)?;
    Some(&t[start..])
}

/// Byte spans of identifier-shaped tokens of a file (comments and strings excluded by the plugin).
pub(crate) fn spans(lang: &str, src: &str) -> Option<Vec<(usize, usize)>> {
    let l = vtok_api::language(lang).ok()?;
    Some(l.identifier_spans(src)?.into_iter().filter(|&(a, b)| src.get(a..b).map_or(false, |t| {
        t.chars().next().map_or(false, |c| c.is_alphabetic() || c == '_' || c == '$') && t.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '$')
    })).collect())
}

/// All occurrences worth resolving. `src` must use `\n` newlines (as the plugin saw it).
pub(crate) fn occurrences(lang: &str, src: &str) -> Vec<Occ> {
    let Some(sp) = spans(lang, src) else { return vec![] };
    let mut starts = vec![0usize];
    for (i, b) in src.bytes().enumerate() { if b == b'\n' { starts.push(i + 1); } }
    let mut out = vec![];
    for (a, b) in sp {
        let name = &src[a..b];
        let head = &src[..a];
        let tail = src[b..].trim_start_matches([' ', '\t']);
        let ht = head.trim_end();
        // definitions (`fn name`, `class Name`) and keyword-like tokens are not uses
        if ident_before(ht).map_or(false, |w| ht.len() == head.trim_end().len() && DEF_KW.contains(&w) && !ht.ends_with("::")) { continue; }
        if tail.starts_with('!') && !tail.starts_with("!=") { continue; }
        let after = tail.trim_start();
        // `key: value` in struct literals / object literals / named arguments, `name=value` keyword arguments
        let field_key = (after.starts_with(':') && !after.starts_with("::")) || (after.starts_with('=') && !after.starts_with("==") && !after.starts_with("=>") && lang == "python" && head.trim_end().ends_with([',', '(']));
        let (mut form, mut recv, mut qual) = (Form::Bare, None, vec![]);
        if let Some(h) = ht.strip_suffix('.').filter(|h| !h.ends_with('.')) {
            form = Form::Member;
            let h2 = h.trim_end();
            recv = ident_before(h2).filter(|r| { let rest = &h2[..h2.len() - r.len()]; { let t = rest.trim_end(); !(t.ends_with('.') || t.ends_with("::")) } }).map(String::from);
        } else if ht.ends_with("::") {
            form = Form::Path;
            let mut h = ht;
            while let Some(p) = h.strip_suffix("::") {
                let p = p.trim_end();
                let Some(seg) = ident_before(p) else { break };
                qual.insert(0, seg.to_string());
                h = p[..p.len() - seg.len()].trim_end();
            }
        }
        if field_key && form == Form::Bare { continue; }
        let call = after.starts_with('(') || (after.starts_with("::<") && { let t = &after[3..]; t.find('>').map_or(false, |i| t[i + 1..].trim_start().starts_with('(')) });
        let line = starts.partition_point(|&s| s <= a);
        out.push(Occ { name: name.to_string(), line, form, recv, qual, call, head: after.starts_with("::") && !after.starts_with("::<") });
    }
    out
}
