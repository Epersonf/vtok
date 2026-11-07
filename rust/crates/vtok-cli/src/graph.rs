//! `vtok graph`: a persistent, name-based call graph of a project (functions, methods, types, plus markdown docs), with
//! communities, hubs, path and explain queries. Built from the same symbol and identifier data as `refs`, stored in
//! `.vtok/graph.json`, and rebuilt automatically when a source file is newer than the graph.
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use vtok_api::*;

const MAX_HOMONYMS: usize = 3;

#[derive(Clone)]
pub struct Node { pub file: String, pub name: String, pub a: usize, pub b: usize, pub kind: &'static str }

#[derive(Default)]
pub struct Graph { pub nodes: Vec<Node>, pub edges: Vec<(usize, usize)>, pub files: usize, pub label: Vec<usize> }

fn kind_of(text: &str, id: &str) -> &'static str {
    let w: Vec<&str> = text.split(|c: char| c.is_whitespace() || c == '(' || c == '<').filter(|x| !x.is_empty()).collect();
    if w.iter().take(5).any(|t| matches!(*t, "fn" | "def" | "function")) { if id.contains('.') { "method" } else { "function" } }
    else if w.iter().take(5).any(|t| matches!(*t, "class" | "struct" | "enum" | "trait" | "interface" | "type" | "union")) { "type" }
    else if id.contains('.') { "method" } else { "value" }
}

pub fn build(root: &Path) -> Graph {
    let mut files = vec![];
    crate::refs::walk(root, &mut files);
    let mut g = Graph::default();
    struct F { rel: String, src: String, syms: Vec<(String, usize, usize)>, node_of: Vec<Option<usize>> }
    let mut parsed: Vec<F> = vec![];
    for f in &files {
        let Ok(src) = std::fs::read_to_string(f) else { continue };
        let rel = f.strip_prefix(root).unwrap_or(f).display().to_string();
        let Ok(l) = language_for_path(&rel) else { continue };
        let Ok(syms) = symbols(l.name(), &src) else { continue };
        let lines: Vec<&str> = src.lines().collect();
        let mut node_of = vec![];
        for (id, a, b) in &syms {
            if id == "use" || id.starts_with("use#") || id.starts_with("import") || id.starts_with("mod#") { node_of.push(None); continue; }
            let text = lines.get(a - 1).copied().unwrap_or("");
            let t = text.trim();
            if (t.starts_with("mod ") || t.starts_with("pub mod ")) && t.ends_with(';') { node_of.push(None); continue; }
            g.nodes.push(Node { file: rel.clone(), name: id.split('#').next().unwrap_or(id).to_string(), a: *a, b: *b, kind: kind_of(text, id) });
            node_of.push(Some(g.nodes.len() - 1));
        }
        parsed.push(F { rel, src, syms, node_of });
    }
    g.files = parsed.len();
    // base name -> definitions
    let mut by_base: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, n) in g.nodes.iter().enumerate() { by_base.entry(crate::refs::base(&n.name).to_string()).or_default().push(i); }
    let mut edges: HashSet<(usize, usize)> = HashSet::new();
    for f in &parsed {
        let Ok(l) = language_for_path(&f.rel) else { continue };
        let Ok(ids) = idents_dot(l.name(), &f.src) else { continue };
        let local: HashSet<&str> = f.node_of.iter().flatten().map(|&i| crate::refs::base(&g.nodes[i].name)).collect();
        for (name, ln, dot, call) in ids {
            let Some(defs) = by_base.get(&name) else { continue };
            if defs.len() > MAX_HOMONYMS { continue; }
            // innermost listed symbol that contains the line
            let Some((idx, _)) = f.syms.iter().enumerate().filter(|(i, (_, a, b))| f.node_of[*i].is_some() && *a <= ln && ln <= *b).min_by_key(|(_, (_, a, b))| b - a) else { continue };
            let from = f.node_of[idx].unwrap();
            for &to in defs {
                if to == from || (g.nodes[to].file == f.rel && g.nodes[to].a == ln) { continue; }
                // the usual name-based rules: methods need `.name`, functions need a call, others any mention
                let ok = match g.nodes[to].kind { "method" => dot, "function" => call && !dot, _ => !dot || local.contains(name.as_str()) };
                if ok { edges.insert((from, to)); }
            }
        }
    }
    // markdown: a doc node per file, linked to every unique symbol named in backticks
    let mut docs: Vec<PathBuf> = vec![];
    fn md(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(rd) = std::fs::read_dir(dir) else { return };
        for e in rd.flatten() {
            let p = e.path();
            let name = e.file_name().to_string_lossy().to_string();
            if p.is_dir() { if !name.starts_with('.') && !["target", "node_modules", "venv", "dist", "build", "vendor", "site-packages"].contains(&name.as_str()) && out.len() < 400 { md(&p, out); } }
            else if name.ends_with(".md") && out.len() < 400 { out.push(p); }
        }
    }
    md(root, &mut docs);
    for d in docs {
        let Ok(t) = std::fs::read_to_string(&d) else { continue };
        let rel = d.strip_prefix(root).unwrap_or(&d).display().to_string();
        let mut linked: Vec<usize> = vec![];
        for (i, seg) in t.split('`').enumerate() {
            if i % 2 == 0 { continue; }
            let w = seg.trim().trim_end_matches("()");
            let w = w.rsplit(['.', ':']).next().unwrap_or(w);
            if let Some(defs) = by_base.get(w) { if defs.len() == 1 && !linked.contains(&defs[0]) { linked.push(defs[0]); } }
        }
        if linked.is_empty() { continue; }
        g.nodes.push(Node { file: rel.clone(), name: rel, a: 1, b: t.lines().count().max(1), kind: "doc" });
        let id = g.nodes.len() - 1;
        for to in linked { edges.insert((id, to)); }
    }
    g.edges = edges.into_iter().collect();
    g.edges.sort();
    g.label = communities(g.nodes.len(), &g.edges);
    g
}

/// Deterministic label propagation over the undirected graph; labels are renumbered by community size.
fn communities(n: usize, edges: &[(usize, usize)]) -> Vec<usize> {
    // god nodes (shared types and helpers used everywhere) glue unrelated code together: leave them out of the grouping
    let mut deg = vec![0usize; n];
    for &(a, b) in edges { deg[a] += 1; deg[b] += 1; }
    let cap = (n / 12).max(25);
    let mut adj: Vec<Vec<usize>> = vec![vec![]; n];
    for &(a, b) in edges { if deg[a] <= cap && deg[b] <= cap { adj[a].push(b); adj[b].push(a); } }
    let mut lab: Vec<usize> = (0..n).collect();
    for _ in 0..12 {
        let mut changed = false;
        for v in 0..n {
            if adj[v].is_empty() { continue; }
            let mut cnt: BTreeMap<usize, usize> = BTreeMap::new();
            for &u in &adj[v] { *cnt.entry(lab[u]).or_default() += 1; }
            let best = cnt.iter().max_by(|a, b| a.1.cmp(b.1).then(b.0.cmp(a.0))).map(|(l, _)| *l).unwrap_or(lab[v]);
            if best != lab[v] { lab[v] = best; changed = true; }
        }
        if !changed { break; }
    }
    let mut size: HashMap<usize, usize> = HashMap::new();
    for &l in &lab { *size.entry(l).or_default() += 1; }
    let mut order: Vec<(usize, usize)> = size.into_iter().collect();
    order.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let re: HashMap<usize, usize> = order.iter().enumerate().map(|(i, (l, _))| (*l, i)).collect();
    lab.iter().map(|l| re[l]).collect()
}

impl Graph {
    fn degree(&self) -> Vec<usize> { let mut d = vec![0; self.nodes.len()]; for &(a, b) in &self.edges { d[a] += 1; d[b] += 1; } d }
    fn short(&self, i: usize) -> String { let n = &self.nodes[i]; format!("{} {}:{}", n.name, n.file, n.a) }
    fn community_name(&self, c: usize) -> String {
        let deg = self.degree();
        let members: Vec<usize> = (0..self.nodes.len()).filter(|&i| self.label[i] == c).collect();
        let mut dirs: BTreeMap<String, usize> = BTreeMap::new();
        for &i in &members { let f = &self.nodes[i].file; *dirs.entry(f.rsplit_once('/').map_or(".", |x| x.0).to_string()).or_default() += 1; }
        let dir = dirs.into_iter().max_by_key(|(_, n)| *n).map(|x| x.0).unwrap_or_default();
        let top = members.iter().max_by_key(|&&i| deg[i]).map(|&i| crate::refs::base(&self.nodes[i].name).to_string()).unwrap_or_default();
        format!("{dir} / {top}")
    }
    fn find(&self, q: &str) -> Vec<usize> {
        let exact: Vec<usize> = (0..self.nodes.len()).filter(|&i| self.nodes[i].name == q).collect();
        if !exact.is_empty() { return exact; }
        (0..self.nodes.len()).filter(|&i| crate::refs::base(&self.nodes[i].name) == q).collect()
    }

    pub fn summary(&self) -> String {
        let ncomm = self.label.iter().max().map_or(0, |m| m + 1);
        let multi = (0..ncomm).filter(|&c| self.label.iter().filter(|&&l| l == c).count() > 1).count();
        format!("graph: {} nodes ({} files), {} edges, {} communities with 2+ members\n", self.nodes.len(), self.files, self.edges.len(), multi)
    }

    pub fn hubs(&self, n: usize) -> String {
        let deg = self.degree();
        let mut idx: Vec<usize> = (0..self.nodes.len()).collect();
        idx.sort_by_key(|&i| std::cmp::Reverse(deg[i]));
        let mut out = String::from("# hubs: most connected symbols (callers + callees)\n");
        for &i in idx.iter().take(n).filter(|&&i| deg[i] > 0) { out.push_str(&format!("{:>3} {} [{}]\n", deg[i], self.short(i), self.nodes[i].kind)); }
        out
    }

    pub fn communities_text(&self, n: usize) -> String {
        let ncomm = self.label.iter().max().map_or(0, |m| m + 1);
        let deg = self.degree();
        let mut out = String::from("# communities (tightly connected groups of symbols)\n");
        let mut shown = 0;
        for c in 0..ncomm {
            let mut m: Vec<usize> = (0..self.nodes.len()).filter(|&i| self.label[i] == c).collect();
            if m.len() < 2 { continue; }
            if shown >= n { break; }
            shown += 1;
            m.sort_by_key(|&i| std::cmp::Reverse(deg[i]));
            let files: HashSet<&str> = m.iter().map(|&i| self.nodes[i].file.as_str()).collect();
            out.push_str(&format!("C{c} {} — {} symbols in {} files: {}\n", self.community_name(c), m.len(), files.len(), m.iter().take(6).map(|&i| crate::refs::base(&self.nodes[i].name).to_string()).collect::<Vec<_>>().join(", ")));
        }
        out
    }

    pub fn explain(&self, q: &str) -> String {
        let hits = self.find(q);
        if hits.is_empty() { return format!("# no symbol named {q:?} in the graph\n"); }
        let mut out = String::new();
        for &i in hits.iter().take(4) {
            let n = &self.nodes[i];
            out.push_str(&format!("{} [{}] {}:{}-{}  community C{} ({})\n", n.name, n.kind, n.file, n.a, n.b, self.label[i], self.community_name(self.label[i])));
            let callers: Vec<String> = self.edges.iter().filter(|e| e.1 == i).map(|e| self.short(e.0)).collect();
            let callees: Vec<String> = self.edges.iter().filter(|e| e.0 == i).map(|e| self.short(e.1)).collect();
            let lst = |v: &Vec<String>| if v.is_empty() { "-".to_string() } else if v.len() > 12 { format!("{}, … +{} more", v[..12].join(", "), v.len() - 12) } else { v.join(", ") };
            out.push_str(&format!("  used by ({}): {}\n  uses ({}): {}\n", callers.len(), lst(&callers), callees.len(), lst(&callees)));
        }
        out
    }

    pub fn path(&self, from: &str, to: &str) -> String {
        let (a, b) = (self.find(from), self.find(to));
        if a.is_empty() || b.is_empty() { return format!("# {} not found in the graph\n", if a.is_empty() { from } else { to }); }
        let goal: HashSet<usize> = b.iter().copied().collect();
        for directed in [true, false] {
            let mut prev: HashMap<usize, usize> = HashMap::new();
            let mut q: VecDeque<usize> = a.iter().copied().collect();
            let seen_start: HashSet<usize> = a.iter().copied().collect();
            let mut seen = seen_start.clone();
            let mut hit = None;
            while let Some(v) = q.pop_front() {
                if goal.contains(&v) { hit = Some(v); break; }
                for &(x, y) in &self.edges {
                    let nxt = if x == v { Some(y) } else if !directed && y == v { Some(x) } else { None };
                    if let Some(w) = nxt { if seen.insert(w) { prev.insert(w, v); q.push_back(w); } }
                }
            }
            if let Some(mut v) = hit {
                let mut chain = vec![v];
                while let Some(&p) = prev.get(&v) { chain.push(p); v = p; }
                chain.reverse();
                let arrow = if directed { " -> " } else { " -- " };
                return format!("# {} hops{}\n{}\n", chain.len() - 1, if directed { "" } else { " (ignoring call direction)" }, chain.iter().map(|&i| self.short(i)).collect::<Vec<_>>().join(arrow));
            }
        }
        format!("# no connection between {from} and {to}\n")
    }

    fn to_json(&self) -> Value {
        json!({"v": 1, "files": self.files,
               "nodes": self.nodes.iter().map(|n| json!([n.file, n.name, n.a, n.b, n.kind])).collect::<Vec<_>>(),
               "edges": self.edges.iter().map(|e| json!([e.0, e.1])).collect::<Vec<_>>()})
    }

    fn from_json(v: &Value) -> Option<Graph> {
        let kind = |s: &str| -> &'static str { match s { "function" => "function", "method" => "method", "type" => "type", "doc" => "doc", _ => "value" } };
        let nodes: Vec<Node> = v.get("nodes")?.as_array()?.iter().filter_map(|n| { let a = n.as_array()?; Some(Node { file: a.first()?.as_str()?.into(), name: a.get(1)?.as_str()?.into(), a: a.get(2)?.as_u64()? as usize, b: a.get(3)?.as_u64()? as usize, kind: kind(a.get(4)?.as_str()?) }) }).collect();
        let edges: Vec<(usize, usize)> = v.get("edges")?.as_array()?.iter().filter_map(|e| { let a = e.as_array()?; Some((a.first()?.as_u64()? as usize, a.get(1)?.as_u64()? as usize)) }).collect();
        if edges.iter().any(|&(a, b)| a >= nodes.len() || b >= nodes.len()) { return None; }
        let label = communities(nodes.len(), &edges);
        Some(Graph { files: v.get("files")?.as_u64()? as usize, nodes, edges, label })
    }
}

fn newest_source(root: &Path) -> std::time::SystemTime {
    let mut files = vec![];
    crate::refs::walk(root, &mut files);
    files.iter().filter_map(|f| std::fs::metadata(f).and_then(|m| m.modified()).ok()).max().unwrap_or(std::time::UNIX_EPOCH)
}

fn graph_path(root: &Path) -> PathBuf { root.join(".vtok/graph.json") }

/// Load the stored graph, or build (and store) it when missing or older than any source file.
pub fn load_or_build(root: &Path) -> Graph {
    let p = graph_path(root);
    if let Ok(meta) = std::fs::metadata(&p) {
        if meta.modified().map_or(false, |m| m >= newest_source(root)) {
            if let Some(g) = std::fs::read_to_string(&p).ok().and_then(|t| serde_json::from_str::<Value>(&t).ok()).and_then(|v| Graph::from_json(&v)) { return g; }
        }
    }
    let g = build(root);
    save(root, &g);
    g
}

fn save(root: &Path, g: &Graph) {
    let p = graph_path(root);
    if let Some(d) = p.parent() { let _ = std::fs::create_dir_all(d); let _ = std::fs::write(d.join(".gitignore"), "*\n"); }
    let _ = std::fs::write(&p, g.to_json().to_string());
}

const HOOK_BEGIN: &str = "# vtok:graph begin";
const HOOK_END: &str = "# vtok:graph end";

fn install_hook(root: &Path) -> std::result::Result<PathBuf, String> {
    let hooks = root.join(".git/hooks");
    if !hooks.is_dir() { return Err("no .git/hooks here (run from the repository root)".into()); }
    let p = hooks.join("post-commit");
    let old = std::fs::read_to_string(&p).unwrap_or_else(|_| "#!/bin/sh\n".to_string());
    let block = format!("{HOOK_BEGIN}\n(vtok graph build >/dev/null 2>&1 &)\n{HOOK_END}\n");
    let new = match (old.find(HOOK_BEGIN), old.find(HOOK_END)) {
        (Some(a), Some(b)) if a < b => format!("{}{}{}", &old[..a], block, old[b + HOOK_END.len()..].trim_start_matches('\n')),
        _ => format!("{}{}{}", old, if old.ends_with('\n') { "" } else { "\n" }, block),
    };
    std::fs::write(&p, new).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    { use std::os::unix::fs::PermissionsExt; let _ = std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)); }
    Ok(p)
}

pub fn text_for(action: &str, a: Option<&str>, b: Option<&str>, root: &Path) -> std::result::Result<String, String> {
    let g = load_or_build(root);
    Ok(match action {
        "build" => { let g = build(root); save(root, &g); format!("{}{}{}", g.summary(), g.hubs(5), g.communities_text(5)) }
        "hubs" => g.hubs(a.and_then(|s| s.parse().ok()).unwrap_or(15)),
        "communities" => g.communities_text(a.and_then(|s| s.parse().ok()).unwrap_or(12)),
        "explain" => g.explain(a.ok_or("explain needs a symbol name")?),
        "path" => g.path(a.ok_or("path needs two symbol names")?, b.ok_or("path needs two symbol names")?),
        other => return Err(format!("unknown graph action {other:?} (build, explain, path, hubs, communities)")),
    })
}

pub fn run(args: &[String]) -> i32 {
    let pos: Vec<&String> = args.iter().filter(|a| !a.starts_with("--")).collect();
    let root = args.iter().position(|a| a == "--dir").and_then(|i| args.get(i + 1)).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
    let pos: Vec<&String> = pos.into_iter().filter(|p| Some(p.as_str()) != args.iter().position(|a| a == "--dir").and_then(|i| args.get(i + 1)).map(|s| s.as_str())).collect();
    let Some(action) = pos.first() else { eprintln!("usage: vtok graph build|explain NAME|path A B|hubs [N]|communities [N]|hook [--dir D]"); return 2 };
    if action.as_str() == "hook" {
        return match install_hook(&root) { Ok(p) => { println!("{}: rebuilds the graph after each commit", p.display()); 0 } Err(e) => { eprintln!("{e}"); 1 } };
    }
    match text_for(action, pos.get(1).map(|s| s.as_str()), pos.get(2).map(|s| s.as_str()), &root) {
        Ok(t) => { print!("{t}"); 0 }
        Err(e) => { eprintln!("{e}"); 2 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("vtok-graph-{tag}-{}", std::process::id()));
        std::fs::create_dir_all(d.join("pkg")).unwrap();
        std::fs::write(d.join("pkg/a.py"), "def leaf(x):\n    return x\n\ndef mid(x):\n    return leaf(x)\n").unwrap();
        std::fs::write(d.join("pkg/b.py"), "from a import mid\n\ndef top():\n    return mid(1)\n\nclass K:\n    def m(self):\n        return top()\n\ndef lone():\n    return 1\n").unwrap();
        std::fs::write(d.join("NOTES.md"), "See `top()` and `K.m`.\n").unwrap();
        d
    }

    #[test]
    fn edges_follow_calls_and_docs_link_to_symbols() {
        let d = fixture("a");
        let g = build(&d);
        let id = |n: &str| g.nodes.iter().position(|x| x.name == n).unwrap();
        assert!(g.edges.contains(&(id("mid"), id("leaf"))) && g.edges.contains(&(id("top"), id("mid"))) && g.edges.contains(&(id("K.m"), id("top"))));
        assert!(!g.edges.iter().any(|e| e.0 == id("lone") || e.1 == id("lone")));
        assert!(g.nodes.iter().any(|n| n.kind == "doc" && n.name == "NOTES.md"));
        let p = g.path("K.m", "leaf");
        assert!(p.contains("3 hops") && p.contains("K.m") && p.contains("leaf"), "{p}");
        let e = g.explain("mid");
        assert!(e.contains("used by (1): top") && e.contains("uses (1): leaf"), "{e}");
        assert!(g.hubs(3).contains("mid"));
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn stored_graph_round_trips_and_goes_stale() {
        let d = fixture("b");
        let a = load_or_build(&d);
        assert!(d.join(".vtok/graph.json").exists());
        let b = load_or_build(&d);
        assert_eq!((a.nodes.len(), a.edges.len()), (b.nodes.len(), b.edges.len()));
        std::thread::sleep(std::time::Duration::from_millis(1100));
        std::fs::write(d.join("pkg/c.py"), "def extra():\n    return 2\n").unwrap();
        assert_eq!(load_or_build(&d).nodes.len(), a.nodes.len() + 1, "a newer source file triggers a rebuild");
        std::fs::remove_dir_all(&d).ok();
    }
}
