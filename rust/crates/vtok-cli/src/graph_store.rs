//! Storage and command entry points of `vtok graph`: `.vtok/graph.json`, staleness, the git hook, `run`.
use super::edges::EdgeMap;
use super::{build, docs, Graph, Node, CALLS, KINDS, MENTIONS, REFERENCES, RELS, USES_TYPE};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

impl Graph {
    /// Version 3: edges are [from, to, relation index into "rels", score in percent (100 = EXTRACTED)].
    /// Versions 1 and 2 ([from, to] and [from, to, 0|1]) still load, with the relation guessed from the target kind;
    /// `load_or_build` rebuilds them anyway.
    pub(crate) fn to_json(&self) -> Value {
        json!({"v": 3, "files": self.files, "rels": RELS,
               "nodes": self.nodes.iter().map(|n| json!([n.file, n.name, n.a, n.b, n.kind])).collect::<Vec<_>>(),
               "edges": self.edges.iter().zip(&self.rel).zip(&self.score).map(|((e, r), s)| json!([e.0, e.1, r, s])).collect::<Vec<_>>()})
    }

    pub(crate) fn from_json(v: &Value) -> Option<Graph> {
        let kind = |s: &str| -> &'static str { KINDS.iter().copied().find(|k| *k == s).unwrap_or("value") };
        let nodes: Vec<Node> = v.get("nodes")?.as_array()?.iter().filter_map(|n| { let a = n.as_array()?; Some(Node { file: a.first()?.as_str()?.into(), name: a.get(1)?.as_str()?.into(), a: a.get(2)?.as_u64()? as usize, b: a.get(3)?.as_u64()? as usize, kind: kind(a.get(4)?.as_str()?) }) }).collect();
        let v3 = v.get("v").and_then(|x| x.as_u64()).unwrap_or(1) >= 3;
        let names: Vec<String> = v.get("rels").and_then(|r| r.as_array()).map(|r| r.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default();
        let mut m = EdgeMap::new();
        for e in v.get("edges")?.as_array()? {
            let a = e.as_array()?;
            let (x, y) = (a.first()?.as_u64()? as usize, a.get(1)?.as_u64()? as usize);
            if x >= nodes.len() || y >= nodes.len() { return None; }
            let (rel, score) = if v3 {
                let r = names.get(a.get(2)?.as_u64()? as usize).and_then(|n| RELS.iter().position(|k| k == n))? as u8;
                (r, a.get(3)?.as_u64()?.min(100) as u8)
            } else {
                let r = if !nodes[x].is_code() || !nodes[y].is_code() { MENTIONS } else { match nodes[y].kind { "type" => USES_TYPE, "value" => REFERENCES, _ => CALLS } };
                (r, if a.get(2).and_then(|c| c.as_u64()).unwrap_or(0) == 0 { 100 } else { 70 })
            };
            m.insert((x, y, rel), score);
        }
        let mut g = Graph { files: v.get("files")?.as_u64()? as usize, nodes, ..Default::default() };
        g.finish(m);
        Some(g)
    }
}

fn newest_source(root: &Path) -> std::time::SystemTime {
    let mut files = vec![];
    crate::refs::walk(root, &mut files);
    let code = files.iter().filter_map(|f| std::fs::metadata(f).and_then(|m| m.modified()).ok()).max().unwrap_or(std::time::UNIX_EPOCH);
    code.max(docs::newest(root))
}

fn graph_path(root: &Path) -> PathBuf { root.join(".vtok/graph.json") }

/// Load the stored graph, or build (and store) it when missing or older than any source file.
pub fn load_or_build(root: &Path) -> Graph {
    let p = graph_path(root);
    if let Ok(meta) = std::fs::metadata(&p) {
        if meta.modified().map_or(false, |m| m >= newest_source(root)) {
            let v = std::fs::read_to_string(&p).ok().and_then(|t| serde_json::from_str::<Value>(&t).ok());
            // older versions lack relations and scores: rebuild them transparently
            if let Some(g) = v.filter(|v| v.get("v").and_then(|x| x.as_u64()) == Some(3)).and_then(|v| Graph::from_json(&v)) { return g; }
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
    if action == "build" { let g = build(root); save(root, &g); return Ok(format!("{}{}{}", g.summary(), g.hubs(5), g.communities_text(5))); }
    let g = load_or_build(root);
    Ok(match action {
        "report" => { let r = g.report(); let _ = std::fs::write(root.join(".vtok/GRAPH_REPORT.md"), &r); r }
        "html" => { let p = root.join(".vtok/graph.html"); std::fs::write(&p, g.html()).map_err(|e| format!("{}: {e}", p.display()))?; format!("{}\n", p.display()) }
        "hubs" => g.hubs(a.and_then(|s| s.parse().ok()).unwrap_or(15)),
        "communities" => g.communities_text(a.and_then(|s| s.parse().ok()).unwrap_or(12)),
        "explain" => g.explain(a.ok_or("explain needs a symbol name")?),
        "path" => g.path(a.ok_or("path needs two symbol names")?, b.ok_or("path needs two symbol names")?),
        other => return Err(format!("unknown graph action {other:?} (build, explain, path, hubs, communities, report, html)")),
    })
}

pub fn run(args: &[String]) -> i32 {
    let pos: Vec<&String> = args.iter().filter(|a| !a.starts_with("--")).collect();
    let dir_val = args.iter().position(|a| a == "--dir").and_then(|i| args.get(i + 1));
    let root = dir_val.map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
    let pos: Vec<&String> = pos.into_iter().filter(|p| Some(*p) != dir_val).collect();
    let Some(action) = pos.first() else { eprintln!("usage: vtok graph build|explain NAME|path A B|hubs [N]|communities [N]|report|html|hook [--dir D]"); return 2 };
    if action.as_str() == "hook" {
        return match install_hook(&root) { Ok(p) => { println!("{}: rebuilds the graph after each commit", p.display()); 0 } Err(e) => { eprintln!("{e}"); 1 } };
    }
    match text_for(action, pos.get(1).map(|s| s.as_str()), pos.get(2).map(|s| s.as_str()), &root) {
        Ok(t) => { print!("{t}"); 0 }
        Err(e) => { eprintln!("{e}"); 2 }
    }
}
