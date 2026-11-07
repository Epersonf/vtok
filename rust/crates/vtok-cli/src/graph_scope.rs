//! What each file can see: names it imports (and from which files), glob imports, module aliases, re-exports.
//! Rust lives in graph_scope_rs.rs; this file holds the shared shape, path helpers and the Python, TypeScript /
//! JavaScript, Go, Java and C-family import readers. A strict file can only use a bare name that is defined in it or
//! imported (Rust, Python, ES modules, Go, Java); other files fall back to weaker evidence (directory, import words).
use super::edges::Parsed;
use super::scope_langs as langs;
use std::collections::{HashMap, HashSet};

#[derive(Default, Debug)]
pub(crate) struct Scope {
    /// local name -> (name at the definition, files it comes from; empty = an external library)
    pub names: HashMap<String, (String, Vec<usize>)>,
    pub globs: Vec<usize>,
    /// module alias usable as a qualifier (`alias::f`, `alias.f`) -> its files
    pub mods: HashMap<String, Vec<usize>>,
    pub reexports: Vec<usize>,
    pub strict: bool,
    /// identifier words of the import lines (weak evidence for non-strict languages)
    pub words: HashSet<String>,
    /// Rust crate of the file and the crates it may use; module tree (parent, `mod x;` children, crate root)
    pub krate: Option<usize>,
    pub deps: HashSet<usize>,
    pub parent: Option<usize>,
    pub children: HashMap<String, usize>,
    pub root: Option<usize>,
    /// names this file re-exports (`pub use a::b`)
    pub pub_names: HashSet<String>,
}

pub(crate) fn dir_of(file: &str) -> &str { file.rsplit_once('/').map_or("", |x| x.0) }
pub(crate) fn stem_of(file: &str) -> &str { let n = file.rsplit('/').next().unwrap_or(file); n.split('.').next().unwrap_or(n) }

/// `a/b/../c` -> `a/c`
pub(crate) fn normalize(p: &str) -> String {
    let mut out: Vec<&str> = vec![];
    for s in p.split('/') { match s { "" | "." => {} ".." => { out.pop(); } s => out.push(s) } }
    out.join("/")
}

pub(crate) struct Files { pub by_path: HashMap<String, usize>, pub by_dir: HashMap<String, Vec<usize>> }

impl Files {
    pub(crate) fn new(parsed: &[Parsed]) -> Files {
        let mut f = Files { by_path: HashMap::new(), by_dir: HashMap::new() };
        for (i, p) in parsed.iter().enumerate() { f.by_path.insert(p.rel.clone(), i); f.by_dir.entry(dir_of(&p.rel).to_string()).or_default().push(i); }
        f
    }
    /// Files whose path without extension is `base` or ends with `/base` (`base` may itself be a suffix).
    pub(crate) fn module(&self, parsed: &[Parsed], base: &str, exts: &[&str]) -> Vec<usize> {
        if let Some(&i) = exts.iter().find_map(|e| self.by_path.get(&format!("{base}{e}"))) { return vec![i]; }
        let tail = format!("/{base}");
        parsed.iter().enumerate().filter(|(_, p)| exts.iter().any(|e| p.rel.strip_suffix(e).map_or(false, |s| s.ends_with(&tail) || s == base))).map(|(i, _)| i).collect()
    }
    pub(crate) fn dir_suffix(&self, d: &str) -> Vec<usize> {
        let tail = format!("/{d}");
        self.by_dir.iter().filter(|(k, _)| k.as_str() == d || k.ends_with(&tail)).flat_map(|(_, v)| v.iter().copied()).collect()
    }
}

pub(crate) fn import_words(src: &str) -> HashSet<String> {
    const KW: &[&str] = &["import ", "from ", "use ", "pub use ", "require", "#include", "using ", "package ", "mod ", "pub mod ", "@import", "export ", "include "];
    let mut out = HashSet::new();
    for l in src.lines() {
        let t = l.trim_start();
        if KW.iter().any(|k| t.starts_with(k)) || l.contains("require(") || l.contains(" from '") || l.contains(" from \"") {
            for w in l.split(|c: char| !(c.is_alphanumeric() || c == '_')).filter(|w| !w.is_empty()) { out.insert(w.to_string()); }
        }
    }
    out
}

pub(crate) fn build(root: &std::path::Path, parsed: &[Parsed]) -> Vec<Scope> {
    let fs = Files::new(parsed);
    let mut out: Vec<Scope> = parsed.iter().map(|p| Scope { words: import_words(&p.src), ..Default::default() }).collect();
    for (i, p) in parsed.iter().enumerate() {
        match p.lang {
            "python" => { out[i].strict = true; langs::python(i, parsed, &fs, &mut out[i]) }
            "typescript" => langs::typescript(i, parsed, &fs, &mut out[i]),
            "rust" => {}
            l => langs::package_dir(i, parsed, &fs, &mut out[i], l),
        }
    }
    super::scope_rs::build(root, parsed, &fs, &mut out);
    out
}
