//! `vtok refs NAME [DIR]`: where a symbol is defined and where its name is used, across a project.
//!
//! Name-based, not type-resolved: every identifier token equal to NAME counts, grouped by the symbol that contains it.
//! Files that do not contain NAME as text are never parsed, so a query costs one pass of substring checks plus a few parses.
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use vtok_api::*;

const SKIP_DIRS: &[&str] = &["target", "node_modules", "venv", "dist", "build", "__pycache__", "vendor", "site-packages"];
const MAX_FILES: usize = 5000;
const MAX_BYTES: u64 = 1_000_000;

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    let mut entries: Vec<_> = rd.flatten().collect();
    entries.sort_by_key(|e| e.file_name());
    for e in entries {
        if out.len() >= MAX_FILES { return; }
        let name = e.file_name().to_string_lossy().to_string();
        let path = e.path();
        let Ok(ft) = e.file_type() else { continue };
        if ft.is_dir() {
            if name.starts_with('.') || SKIP_DIRS.contains(&name.as_str()) { continue; }
            walk(&path, out);
        } else if ft.is_file() && language_for_path(&name).is_ok() && e.metadata().map_or(false, |m| m.len() <= MAX_BYTES) {
            out.push(path);
        }
    }
}

fn base(id: &str) -> &str {
    let id = id.split('#').next().unwrap_or(id);
    id.rsplit('.').next().unwrap_or(id)
}

/// `class Name`, `def name`, `pub fn name`, `export const name` ...: a line that defines `name` (checked on the text).
fn defines(line: &str, name: &str) -> bool {
    let mut w = line.trim().split(|c: char| c.is_whitespace() || c == '(' || c == '<' || c == ':' || c == '{' || c == '=' || c == ';').filter(|x| !x.is_empty());
    while let Some(t) = w.next() {
        if matches!(t, "pub" | "export" | "default" | "async" | "unsafe" | "abstract" | "declare" | "extern" | "final" | "static") || t.starts_with("pub") || t.ends_with(')') { continue; }
        return matches!(t, "fn" | "struct" | "enum" | "trait" | "type" | "const" | "mod" | "class" | "def" | "function" | "interface" | "let" | "var" | "union") && w.next() == Some(name);
    }
    false
}

pub struct Report { pub text: String, pub raw_tokens: usize }

/// Defs and uses of `name` under `root`, as compact lines, within `budget` tokens.
pub fn find(name: &str, root: &Path, budget: usize) -> Report {
    let mut files = vec![];
    walk(root, &mut files);
    let mut defs: Vec<String> = vec![];
    // (file, enclosing symbol, range) -> lines
    let mut uses: BTreeMap<(String, String, String), Vec<usize>> = BTreeMap::new();
    let (mut nfiles, mut skipped, mut raw_tokens) = (0usize, 0usize, 0usize);
    for f in &files {
        let Ok(src) = std::fs::read_to_string(f) else { continue };
        if !src.contains(name) { continue; }
        let rel = f.strip_prefix(root).unwrap_or(f).display().to_string();
        let Ok(l) = language_for_path(&rel) else { continue };
        let (Ok(syms), Ok(ids)) = (symbols(l.name(), &src), idents(l.name(), &src)) else { skipped += 1; continue };
        raw_tokens += ntok(&src);
        let text_lines: Vec<&str> = src.lines().collect();
        let hits: Vec<usize> = ids.iter().filter(|(n, _)| n == name).map(|(_, ln)| *ln).collect();
        if hits.is_empty() { continue; }
        nfiles += 1;
        let span = |a: usize, b: usize| if a == b { format!("L{a}") } else { format!("L{a}-{b}") };
        let is_def = |id: &str| base(id) == name && id != "use" && !id.starts_with("use#") && !id.starts_with("import");
        for (id, a, b) in &syms { if is_def(id) { defs.push(format!("def {rel} {} {}", id.split('#').next().unwrap_or(id), span(*a, *b))); } }
        for ln in hits {
            let text = text_lines.get(ln - 1).copied().unwrap_or("");
            let encl_def = syms.iter().any(|(id, a, _)| is_def(id) && *a == ln);
            if defines(text, name) && !encl_def { defs.push(format!("def {rel} {name} {} (outside any listed symbol, e.g. a class shell)", span(ln, ln))); continue; }
            // smallest symbol containing the line
            let enc = syms.iter().filter(|(_, a, b)| *a <= ln && ln <= *b).min_by_key(|(_, a, b)| b - a);
            match enc {
                Some((id, a, b)) if is_def(id) && *a == ln => {} // the definition's own name
                Some((id, a, b)) => uses.entry((rel.clone(), id.split('#').next().unwrap_or(id).to_string(), span(*a, *b))).or_default().push(ln),
                None => uses.entry((rel.clone(), "(top level)".into(), String::new())).or_default().push(ln),
            }
        }
    }
    let nuses: usize = uses.values().map(|v| v.len()).sum();
    let mut out = format!("# refs {name}: {} def, {nuses} uses in {nfiles} files (by name, not type-resolved)\n", defs.len());
    if defs.is_empty() { out.push_str("# no definition found in this tree (external, generated, or a different spelling)\n"); }
    if defs.len() > 1 { out.push_str(&format!("# {} definitions share this name: uses are not told apart\n", defs.len())); }
    if skipped > 0 { out.push_str(&format!("# {skipped} files skipped (syntax errors)\n")); }
    for d in &defs { out.push_str(d); out.push('\n'); }
    let mut shown = 0;
    let total = uses.len();
    for ((file, sym, range), lines) in &uses {
        let ls: Vec<String> = lines.iter().map(|l| format!("L{l}")).collect();
        let line = format!("use {file} {sym}{}{} @ {}\n", if range.is_empty() { "" } else { " " }, range, ls.join(","));
        if ntok(&(out.clone() + &line)) > budget { break; }
        out.push_str(&line);
        shown += 1;
    }
    if shown < total { out.push_str(&format!("# {} more use sites not shown (raise max_tokens or narrow with a directory)\n", total - shown)); }
    Report { text: out, raw_tokens }
}

struct FileInfo { rel: String, lines: usize, syms: Vec<(String, usize, usize, String)> }

/// A project overview within a token budget: files ranked by how much the rest of the project refers to their symbols
/// (name-based, so homonyms are merged), the most referenced symbols per file with their line ranges and use counts, and
/// the first source line (the signature) of the very top ones.
pub fn map(root: &Path, budget: usize) -> Report {
    let mut files = vec![];
    walk(root, &mut files);
    let mut infos: Vec<FileInfo> = vec![];
    let mut counts: std::collections::HashMap<String, Vec<(usize, usize, usize, usize)>> = std::collections::HashMap::new();
    let mut raw_tokens = 0;
    for f in &files {
        let Ok(src) = std::fs::read_to_string(f) else { continue };
        let rel = f.strip_prefix(root).unwrap_or(f).display().to_string();
        let Ok(l) = language_for_path(&rel) else { continue };
        let (Ok(syms), Ok(ids)) = (symbols(l.name(), &src), idents_dot(l.name(), &src)) else { continue };
        raw_tokens += ntok(&src);
        let fidx = infos.len();
        // (plain mentions, plain calls, `.name` accesses)
        let mut here: std::collections::HashMap<String, (usize, usize, usize)> = std::collections::HashMap::new();
        for (n, _, dot, call) in ids { let e = here.entry(n).or_default(); if dot { e.2 += 1 } else if call { e.1 += 1 } else { e.0 += 1 } }
        for (n, (plain, calls, dot)) in here { counts.entry(n).or_default().push((fidx, plain, calls, dot)); }
        let text: Vec<&str> = src.lines().collect();
        let syms = syms.into_iter()
            .filter(|(id, _, _)| id != "use" && !id.starts_with("use#") && !id.starts_with("import") && !id.starts_with("tests."))
            .map(|(id, a, b)| { let sig = text.get(a - 1).map_or(String::new(), |t| t.trim().to_string()); (id.split('#').next().unwrap_or(&id).to_string(), a, b, sig) })
            // only declarations (fn, class, struct, def, export ...): module-level variables and statement groups are not an API
            .filter(|(id, _, _, sig)| defines(sig, base(id)))
            .collect();
        infos.push(FileInfo { rel, lines: src.lines().count(), syms });
    }
    // a name's weight = its uses in files that do NOT define it, so std-style names defined in many files (text, parse) and locals fade
    let mut def_files: std::collections::HashMap<String, std::collections::HashSet<usize>> = std::collections::HashMap::new();
    for (i, fi) in infos.iter().enumerate() { for (id, ..) in &fi.syms { def_files.entry(base(id).to_string()).or_default().insert(i); } }
    // plain functions/types count plain mentions; methods (`Type.name`) count `.name` accesses: so `map` the function is not
    // confused with `.map(` of every iterator
    let is_fn: std::collections::HashMap<(usize, String), bool> = infos.iter().enumerate().flat_map(|(i, fi)| fi.syms.iter().map(move |(id, _, _, sig)| ((i, id.clone()), sig.contains("fn ") || sig.contains("def ") || sig.contains("function ")))).collect();
    let fn_names: std::collections::HashSet<String> = is_fn.iter().filter(|(_, v)| **v).map(|((_, id), _)| base(id).to_string()).collect();
    let uses = |id: &str| -> usize {
        let b = base(id);
        let none = std::collections::HashSet::new();
        let d = def_files.get(b).unwrap_or(&none);
        let (method, func) = (id.contains('.'), fn_names.contains(b));
        counts.get(b).map_or(0, |v| v.iter().filter(|c| !d.contains(&c.0)).map(|(_, plain, calls, dot)| if method { *dot } else if func { *calls } else { plain + calls }).sum())
    };
    let mut ranked: Vec<(usize, &FileInfo, Vec<(usize, &(String, usize, usize, String))>)> = infos.iter().map(|fi| {
        let mut ss: Vec<_> = fi.syms.iter().map(|s| (uses(&s.0), s)).filter(|(u, s)| *u > 0 && s.0.chars().count() > 1).collect();
        // homonyms share their uses, so divide by the number of files defining the name
        let w = |u: usize, id: &str| u / def_files.get(base(id)).map_or(1, |d| d.len().max(1));
        ss.sort_by_key(|(u, s)| (std::cmp::Reverse(w(*u, &s.0)), s.1));
        let mut seen = std::collections::HashSet::new();
        ss.retain(|(_, s)| seen.insert(s.0.clone()));
        (ss.iter().map(|(u, s)| w(*u, &s.0)).sum::<usize>(), fi, ss)
    }).collect();
    ranked.sort_by_key(|(sc, fi, _)| (std::cmp::Reverse(*sc), fi.rel.clone()));
    let nsyms: usize = infos.iter().map(|f| f.syms.len()).sum();
    // every file gets a line; symbols are added round by round (most used first, then biggest) until the budget is spent
    let mut order: Vec<usize> = (0..ranked.len()).collect();
    order.sort_by_key(|&k| (std::cmp::Reverse(ranked[k].0), ranked[k].1.rel.clone()));
    let cand: Vec<Vec<(usize, &(String, usize, usize, String))>> = ranked.iter().map(|(_, fi, ss)| {
        let mut v: Vec<(usize, &(String, usize, usize, String))> = ss.clone();
        let mut rest: Vec<&(String, usize, usize, String)> = fi.syms.iter().filter(|s| !v.iter().any(|(_, x)| x.0 == s.0)).collect();
        rest.sort_by_key(|s| std::cmp::Reverse(s.2 - s.1));
        let mut seen = std::collections::HashSet::new();
        for r in rest { if seen.insert(r.0.clone()) && r.0.chars().count() > 1 { v.push((0, r)); } }
        v
    }).collect();
    let render = |k: usize, sigs: usize| -> String {
        let mut out = format!("# map {}: {} files, {nsyms} symbols; xN = uses in files that do not define it (by name)\n", root.display(), infos.len());
        // signatures go to the globally most used symbols: rank by use count across files
        let mut by_use: Vec<(usize, usize)> = vec![];
        for (f, c) in cand.iter().enumerate() { for (n, (u, _)) in c.iter().take(k).enumerate() { if *u > 0 { by_use.push((*u, f * 1000 + n)); } } }
        by_use.sort_by(|a, b| b.cmp(a));
        let sig_set: std::collections::HashSet<usize> = by_use.iter().take(sigs).map(|x| x.1).collect();
        let mut files_sorted: Vec<usize> = (0..ranked.len()).collect();
        files_sorted.sort_by_key(|&f| ranked[f].1.rel.clone());
        let mut cur_dir = String::from("\u{0}");
        for &f in &files_sorted {
            let (_, fi, _) = &ranked[f];
            let (dir, name) = match fi.rel.rsplit_once('/') { Some((d, n)) => (format!("{d}/"), n.to_string()), None => (String::new(), fi.rel.clone()) };
            if dir != cur_dir { if !dir.is_empty() { out.push_str(&format!("{dir}\n")); } cur_dir = dir.clone(); }
            let mut line = format!("{}{name} ({}L):", if dir.is_empty() { "" } else { " " }, fi.lines);
            for (n, (u, (id, a, b, sig))) in cand[f].iter().take(k).enumerate() {
                let span = if a == b { format!("L{a}") } else { format!("L{a}-{b}") };
                let homonyms = def_files.get(base(id)).map_or(0, |d| d.len());
                line.push_str(&format!("{} {id} {span}{}{}", if n == 0 { "" } else { "," }, if *u > 0 { format!(" x{u}") } else { String::new() }, if homonyms > 1 { "~" } else { "" }));
                if sig_set.contains(&(f * 1000 + n)) && !sig.is_empty() { line.push_str(&format!(" `{}`", cap_sig(sig))); }
            }
            out.push_str(&line);
            out.push('\n');
        }
        out
    };
    let maxk = cand.iter().map(|c| c.len()).max().unwrap_or(0);
    let mut best = render(0, 0);
    let mut kbest = 0;
    for k in 1..=maxk { let t = render(k, 0); if ntok(&t) > budget { break; } best = t; kbest = k; }
    for sigs in [4usize, 8, 16, 32] { let t = render(kbest, sigs); if ntok(&t) > budget { break; } best = t; }
    if ntok(&best) > budget { best.push_str("# files alone exceed max_tokens: narrow with a subdirectory\n"); }
    Report { text: best, raw_tokens }
}

fn cap_sig(s: &str) -> String { if s.chars().count() <= 90 { s.to_string() } else { format!("{}…", s.chars().take(90).collect::<String>()) } }

pub fn run_map(args: &[String]) -> i32 {
    let pos: Vec<&String> = args.iter().filter(|a| !a.starts_with("--")).collect();
    let dir = pos.iter().find(|a| a.parse::<usize>().is_err()).map(|a| PathBuf::from(a.as_str())).unwrap_or_else(|| PathBuf::from("."));
    let budget = args.iter().position(|a| a == "--max-tokens").and_then(|i| args.get(i + 1)).and_then(|v| v.parse().ok()).unwrap_or(700);
    let r = map(&dir, budget);
    print!("{}", r.text);
    crate::usage::record("map", r.raw_tokens.max(ntok(&r.text)), ntok(&r.text), &dir.display().to_string());
    0
}

const STOP: &[&str] = &["where", "is", "are", "the", "a", "an", "of", "to", "in", "on", "for", "and", "or", "how", "what", "which", "who", "does", "do", "handled", "handle", "handling", "code", "that", "this", "with", "from", "by", "used", "use", "find", "implemented", "implementation", "logic"];

/// `fooBar_baz2` -> ["foo", "bar", "baz", "2"], lowercased.
fn name_tokens(s: &str) -> Vec<String> {
    let mut out: Vec<String> = vec![];
    let mut cur = String::new();
    let mut prev_lower = false;
    for c in s.chars() {
        if c.is_alphanumeric() {
            if c.is_uppercase() && prev_lower && !cur.is_empty() { out.push(std::mem::take(&mut cur)); }
            prev_lower = c.is_lowercase() || c.is_ascii_digit();
            cur.extend(c.to_lowercase());
        } else if !cur.is_empty() { out.push(std::mem::take(&mut cur)); prev_lower = false; }
    }
    if !cur.is_empty() { out.push(cur); }
    out
}

/// How well query word `w` matches name token `t`: exact 3, one is a prefix of the other (redirect/redirects, auth/authentication) 2.
fn word_match(w: &str, t: &str) -> f64 {
    if w == t { 3.0 } else if w.len() >= 4 && t.len() >= 3 && (t.starts_with(w) || w.starts_with(t)) { 2.0 }
    else if w.len() >= 5 && t.len() >= 5 && w[..5] == t[..5] { 1.5 } else { 0.0 }
}

/// Question-driven symbol search: the words of the question are matched against symbol names (split into words), their parents
/// and the file path. Returns ranked `file symbol L-range` lines ready for `vtok chunk`.
pub fn find_symbols(query: &str, root: &Path, budget: usize) -> Report {
    let words: Vec<String> = name_tokens(query).into_iter().filter(|w| !STOP.contains(&w.as_str()) && w.len() > 1).collect();
    let mut files = vec![];
    walk(root, &mut files);
    struct Cand { rel: String, id: String, a: usize, b: usize, name_t: Vec<String>, path_t: Vec<String>, sig_t: Vec<String> }
    let mut cands: Vec<Cand> = vec![];
    let mut raw_tokens = 0;
    for f in &files {
        let Ok(src) = std::fs::read_to_string(f) else { continue };
        let rel = f.strip_prefix(root).unwrap_or(f).display().to_string();
        let Ok(l) = language_for_path(&rel) else { continue };
        let Ok(syms) = symbols(l.name(), &src) else { continue };
        raw_tokens += ntok(&src);
        let text: Vec<&str> = src.lines().collect();
        let path_t = name_tokens(&rel);
        for (id, a, b) in syms {
            if id == "use" || id.starts_with("use#") || id.starts_with("import") || id.starts_with("tests.") || id == "code" || id.starts_with("code#") { continue; }
            let sig = text.get(a - 1).map_or(String::new(), |t| t.trim().to_string());
            let id = id.split('#').next().unwrap_or(&id).to_string();
            cands.push(Cand { name_t: name_tokens(&id), sig_t: name_tokens(&sig), path_t: path_t.clone(), rel: rel.clone(), id, a, b });
        }
    }
    // rarer words weigh more: df = symbols whose name matches the word
    let n = cands.len().max(1) as f64;
    let weights: Vec<f64> = words.iter().map(|w| {
        let df = cands.iter().filter(|c| c.name_t.iter().any(|t| word_match(w, t) > 0.0)).count().max(1) as f64;
        1.0 + (n / df).ln().min(5.0)
    }).collect();
    let mut scored: Vec<(f64, &Cand)> = cands.iter().filter_map(|c| {
        let mut sc = 0.0;
        let mut hit_words = 0;
        for (w, wt) in words.iter().zip(&weights) {
            let by_name = c.name_t.iter().map(|t| word_match(w, t)).fold(0.0, f64::max);
            let by_path = c.path_t.iter().map(|t| word_match(w, t)).fold(0.0, f64::max) * 0.5;
            let by_sig = c.sig_t.iter().map(|t| word_match(w, t)).fold(0.0, f64::max) * 0.25;
            let m = by_name.max(by_path).max(by_sig);
            if m > 0.0 { hit_words += 1; sc += m * wt; }
        }
        // matching several words is worth more than one strong word
        (hit_words > 0).then(|| (sc * (1.0 + 0.5 * (hit_words as f64 - 1.0)), c))
    }).collect();
    scored.sort_by(|x, y| y.0.partial_cmp(&x.0).unwrap_or(std::cmp::Ordering::Equal).then_with(|| x.1.rel.cmp(&y.1.rel)).then(x.1.a.cmp(&y.1.a)));
    let mut out = format!("# find \"{}\": words {:?}; {} symbols matched (by name words, not by meaning)\n", query, words, scored.len());
    let mut shown = 0;
    for (_, c) in scored.iter().take(15) {
        let line = format!("{} {} {}\n", c.rel, c.id, if c.a == c.b { format!("L{}", c.a) } else { format!("L{}-{}", c.a, c.b) });
        if ntok(&(out.clone() + &line)) > budget { break; }
        out.push_str(&line);
        shown += 1;
    }
    if shown == 0 { out.push_str("# nothing matches: try other words (names are matched, not comments or meaning)\n"); }
    Report { text: out, raw_tokens }
}

pub fn run_find(args: &[String]) -> i32 {
    let budget = args.iter().position(|a| a == "--max-tokens").and_then(|i| args.get(i + 1)).and_then(|v| v.parse().ok()).unwrap_or(400);
    let dir_flag = args.iter().position(|a| a == "--dir").and_then(|i| args.get(i + 1)).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
    let q: Vec<&String> = { let mut skip = false; args.iter().filter(|a| { if skip { skip = false; return false; } if *a == "--max-tokens" || *a == "--dir" { skip = true; return false; } !a.starts_with("--") }).collect() };
    if q.is_empty() { eprintln!("usage: vtok find WORDS... [--dir DIR] [--max-tokens N]"); return 2; }
    let query = q.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(" ");
    let r = find_symbols(&query, &dir_flag, budget);
    print!("{}", r.text);
    crate::usage::record("find", r.raw_tokens.max(ntok(&r.text)), ntok(&r.text), &query);
    0
}

pub fn run(args: &[String]) -> i32 {
    let mut pos = args.iter().filter(|a| !a.starts_with("--"));
    let Some(name) = pos.next() else { eprintln!("usage: vtok refs NAME [DIR] [--max-tokens N]"); return 2 };
    let dir = pos.next().map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
    let budget = args.iter().position(|a| a == "--max-tokens").and_then(|i| args.get(i + 1)).and_then(|v| v.parse().ok()).unwrap_or(500);
    let r = find(name, &dir, budget);
    print!("{}", r.text);
    crate::usage::record("refs", r.raw_tokens.max(ntok(&r.text)), ntok(&r.text), name);
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_defs_and_code_uses_only() {
        let d = std::env::temp_dir().join(format!("vtok-refs-{}", std::process::id()));
        std::fs::create_dir_all(d.join("pkg")).unwrap();
        std::fs::write(d.join("pkg/a.py"), "def helper(x):\n    return x + 1\n\n# helper in a comment\nNOTE = \"helper in a string\"\n").unwrap();
        std::fs::write(d.join("pkg/b.py"), "from a import helper\n\ndef run():\n    return helper(2)\n").unwrap();
        std::fs::write(d.join("c.rs"), "fn other() {}\nfn go() { other(); }\n").unwrap();
        let r = find("helper", &d, 500).text;
        assert!(r.contains("1 def") && r.contains("def pkg/a.py helper L1-2"), "{r}");
        assert!(r.contains("use pkg/b.py run L3-4 @ L4"), "{r}");
        assert!(!r.contains("L4 ") || !r.contains("a.py run"), "{r}");
        assert!(!r.contains("@ L5") && !r.contains("L4,") , "comment and string mentions are not uses: {r}");
        let o = find("other", &d, 500).text;
        assert!(o.contains("def c.rs other L1") && o.contains("use c.rs go L2 @ L2"), "{o}");
        assert!(find("missing_name", &d, 500).text.contains("no definition found"));
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn defines_recognises_declarations() {
        assert!(defines("pub(crate) fn run(x: u8)", "run") && defines("class Session(Base):", "Session") && defines("export const run = 1", "run"));
        assert!(!defines("    run(1)", "run") && !defines("let x = run;", "run"));
    }

    #[test]
    fn map_lists_every_file_and_ranks_used_symbols() {
        let d = std::env::temp_dir().join(format!("vtok-map-{}", std::process::id()));
        std::fs::create_dir_all(d.join("pkg")).unwrap();
        std::fs::write(d.join("pkg/core.py"), "def shared(x):\n    return x\n\ndef lonely():\n    return 1\n").unwrap();
        std::fs::write(d.join("pkg/a.py"), "from core import shared\n\ndef a():\n    return shared(1)\n").unwrap();
        std::fs::write(d.join("pkg/b.py"), "from core import shared\n\ndef b():\n    return shared(2) + shared(3)\n").unwrap();
        let m = map(&d, 400).text;
        assert!(m.contains("pkg/") && m.contains("core.py") && m.contains("a.py") && m.contains("b.py"), "{m}");
        let core = m.lines().find(|l| l.contains("core.py")).unwrap();
        assert!(core.contains("shared") && core.contains("x3"), "shared is used 3 times elsewhere: {core}");
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn find_ranks_by_name_words_and_path() {
        let d = std::env::temp_dir().join(format!("vtok-find-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("cookies.py"), "class CookieJar:\n    def get(self):\n        return 1\n\ndef cookiejar_from_dict(d):\n    return d\n").unwrap();
        std::fs::write(d.join("auth.py"), "def basic_auth_str(u):\n    return u\n\ndef unrelated():\n    return 2\n").unwrap();
        let r = find_symbols("where is the cookie jar built from a dict", &d, 300).text;
        let first = r.lines().nth(1).unwrap();
        assert!(first.contains("cookiejar_from_dict") || first.contains("CookieJar"), "{r}");
        assert!(!r.contains("unrelated"), "{r}");
        let a = find_symbols("basic auth", &d, 300).text;
        assert!(a.lines().nth(1).unwrap().contains("basic_auth_str"), "{a}");
        assert!(find_symbols("zzzz", &d, 300).text.contains("nothing matches"));
        assert_eq!(name_tokens("fooBar_baz2"), vec!["foo", "bar", "baz2"]);
        std::fs::remove_dir_all(&d).ok();
    }
}
