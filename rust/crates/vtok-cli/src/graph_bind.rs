//! Local bindings of one symbol's text: names bound by `let`/`const`/`var`, parameters, closure and loop variables,
//! pattern bindings and assignments. A bare name bound locally shadows a function or constant of the same name, so it
//! is not a call or reference. Also the declared or constructed type of a binding (`x: &Graph`, `let g = Graph::new()`,
//! `x = Repo()`), which types the receiver of a later `x.method()`.
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

#[derive(Default)]
pub(crate) struct Bindings { pub locals: HashSet<String>, pub types: HashMap<String, String> }

fn re(cell: &'static OnceLock<Regex>, pat: &str) -> &'static Regex { cell.get_or_init(|| Regex::new(pat).unwrap()) }

const NOT_TYPES: &[&str] = &["mut", "dyn", "impl", "const", "ref", "Option", "Vec", "Box", "Rc", "Arc", "RefCell", "Cell", "Some", "Ok", "Result", "List", "Optional", "Array", "Promise", "Readonly", "HashMap", "BTreeMap", "HashSet", "BTreeSet", "Mutex"];

/// First type-looking name of a type expression: `&mut Graph` -> Graph, `Option<Node>` -> Node.
pub(crate) fn type_name(t: &str) -> Option<String> {
    t.split(|c: char| !(c.is_alphanumeric() || c == '_')).filter(|w| !w.is_empty())
        .find(|w| w.chars().next().map_or(false, |c| c.is_uppercase()) && !NOT_TYPES.contains(w)).map(String::from)
}

fn idents(s: &str, out: &mut HashSet<String>) {
    for w in s.split(|c: char| !(c.is_alphanumeric() || c == '_' || c == '$')).filter(|w| !w.is_empty()) {
        if w.chars().next().map_or(false, |c| c.is_lowercase() || c == '_') && !matches!(w, "mut" | "ref" | "self" | "in" | "of") { out.insert(w.to_string()); }
    }
}

/// The parameter list of the first line(s) of a symbol: text between the first `(` and its matching `)`.
fn params(text: &str) -> &str {
    let s = ["fn ", "def ", "function ", "func "].iter().filter_map(|k| text.find(k)).min().unwrap_or(0);
    let end = text[s..].find('{').map_or(text.len(), |i| s + i);
    let Some(a) = text[s..end].find('(').map(|i| s + i) else { return "" };
    let mut depth = 0;
    for (i, c) in text[a..].char_indices() {
        match c { '(' => depth += 1, ')' => { depth -= 1; if depth == 0 { return &text[a + 1..a + i]; } } _ => {} }
    }
    ""
}

/// The text with comments and string / char literal contents blanked out (a lexical approximation).
fn code_only(lang: &str, text: &str) -> String {
    let hash = matches!(lang, "python" | "ruby");
    let b = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        let skip_to = |from: usize, end: &[u8]| -> usize { let mut j = from; while j < b.len() && !b[j..].starts_with(end) { if b[j] == b'\\' { j += 1; } j += 1; } (j + end.len()).min(b.len()) };
        let j = if (hash && c == b'#') || (!hash && b[i..].starts_with(b"//")) { b[i..].iter().position(|&x| x == b'\n').map_or(b.len(), |k| i + k) }
            else if !hash && b[i..].starts_with(b"/*") { skip_to(i + 2, b"*/") }
            else if c == b'"' || (c == b'`' && lang == "typescript") { skip_to(i + 1, &[c]) }
            else if c == b'\'' && (hash || lang == "typescript" || b.get(i + 2) == Some(&b'\'') || (b.get(i + 1) == Some(&b'\\') && b[i..].iter().skip(2).take(8).any(|&x| x == b'\''))) { skip_to(i + 1, b"'") }
            else { out.push(c as char); i += 1; continue };
        out.push(' ');
        i = j;
    }
    out
}

pub(crate) fn bindings(lang: &str, text: &str) -> Bindings {
    let text = &code_only(lang, text);
    static LET: OnceLock<Regex> = OnceLock::new();
    static TUP: OnceLock<Regex> = OnceLock::new();
    static FOR: OnceLock<Regex> = OnceLock::new();
    static CLO: OnceLock<Regex> = OnceLock::new();
    static PAT: OnceLock<Regex> = OnceLock::new();
    static ASG: OnceLock<Regex> = OnceLock::new();
    static TYPED: OnceLock<Regex> = OnceLock::new();
    static CTOR: OnceLock<Regex> = OnceLock::new();
    let mut b = Bindings::default();
    for c in re(&LET, r"\b(?:let|const|var|val|auto)\s+(?:mut\s+)?(?:ref\s+)?([A-Za-z_$][\w$]*)").captures_iter(text) { b.locals.insert(c[1].to_string()); }
    for c in re(&TUP, r"\b(?:let|const|var)\s+(?:mut\s+)?[(\[{]([^)\]}=]*)[)\]}]").captures_iter(text) { idents(&c[1], &mut b.locals); }
    for c in re(&FOR, r"\bfor\s*\(?\s*(?:const|let|var|mut)?\s*\(?([\w$, ]+?)\)?\s+(?:in|of)\b").captures_iter(text) { idents(&c[1], &mut b.locals); }
    for c in re(&CLO, r"(?:^|[(,=\s])(?:move\s+)?\|([\w\s,:&()<>\[\]'*]{0,120})\|").captures_iter(text) { idents(&c[1].split(':').next().unwrap_or(""), &mut b.locals); }
    for c in re(&PAT, r"\b(?:Some|Ok|Err)\(\s*(?:ref\s+)?(?:mut\s+)?\(?([\w, ]+)\)?\s*\)").captures_iter(text) { idents(&c[1], &mut b.locals); }
    if matches!(lang, "python" | "ruby") || text.contains(" lambda ") {
        for c in re(&ASG, r"(?m)^\s*([A-Za-z_][\w, ]*?)\s*(?::[^=\n]+)?=[^=]|\bas\s+([A-Za-z_]\w*)|\blambda\s+([\w, ]+):").captures_iter(text) {
            for i in 1..=3 { if let Some(m) = c.get(i) { idents(m.as_str(), &mut b.locals); } }
        }
    }
    // parameters: `name: Type` / `Type name` / `name Type` / `name=default`
    for p in params(text).split(',') {
        let p = p.trim().trim_start_matches(['*', '&']).trim_start_matches("mut ");
        if p.is_empty() { continue; }
        let (name, ty) = match p.split_once(':') {
            Some((n, t)) if !t.starts_with(':') => (n.trim().to_string(), type_name(t)),
            _ => {
                let ws: Vec<&str> = p.split(['=', ' ']).filter(|w| !w.is_empty()).collect();
                let n = if matches!(lang, "go" | "python" | "ruby") { ws.first() } else { ws.last() };
                (n.map(|s| s.trim_matches(|c: char| !(c.is_alphanumeric() || c == '_')).to_string()).unwrap_or_default(), if lang == "go" { ws.get(1).and_then(|t| type_name(t)) } else if ws.len() > 1 { type_name(ws[0]) } else { None })
            }
        };
        let name = name.trim_start_matches("mut ").trim().to_string();
        if name.is_empty() || name == "self" { continue; }
        if let Some(t) = ty { b.types.insert(name.clone(), t); }
        b.locals.insert(name);
    }
    for c in re(&TYPED, r"\b(?:let|const|var)\s+(?:mut\s+)?([A-Za-z_$][\w$]*)\s*:\s*([^=;]+)").captures_iter(text) {
        if let Some(t) = type_name(&c[2]) { b.types.insert(c[1].to_string(), t); }
    }
    for c in re(&CTOR, r"\b([A-Za-z_$][\w$]*)\s*(?::[^=]+)?=\s*(?:new\s+)?&?(?:mut\s+)?([A-Z][\w$]*)\s*(?:::\s*\w+\s*)?[({]").captures_iter(text) {
        if !NOT_TYPES.contains(&&c[2]) { b.types.entry(c[1].to_string()).or_insert_with(|| c[2].to_string()); }
    }
    b
}
