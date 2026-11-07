//! `vtok map`: a project overview within a token budget.
use super::uses::defines;
use super::walk::{base, walk};
use super::Report;
use std::path::Path;
use vtok_api::*;

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
        let rel = f.strip_prefix(root).unwrap_or(f).display().to_string().replace(std::path::MAIN_SEPARATOR, "/");
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
    if ntok(&render(0, 0)) > budget {
        // too many files for one line each: summarise by directory, as deep as the budget allows
        let key = |rel: &str, d: usize| -> String {
            let parts: Vec<&str> = rel.split('/').collect();
            let dirs = &parts[..parts.len() - 1];
            if dirs.is_empty() { ".".to_string() } else { format!("{}/", dirs[..d.min(dirs.len())].join("/")) }
        };
        let max_depth = ranked.iter().map(|(_, fi, _)| fi.rel.split('/').count().saturating_sub(1)).max().unwrap_or(1).max(1);
        let mut best_dirs: Option<String> = None;
        let head = format!("# map {}: {} files, {nsyms} symbols; summarised by directory (budget); xN = uses in files that do not define it\n", root.display(), infos.len());
        for d in 1..=max_depth {
            let mut groups: std::collections::BTreeMap<String, (usize, usize, Vec<(usize, String)>)> = std::collections::BTreeMap::new();
            for (_, fi, ss) in &ranked {
                let g = groups.entry(key(&fi.rel, d)).or_default();
                g.0 += 1; g.1 += fi.lines;
                for (u, s) in ss { g.2.push((*u, s.0.clone())); }
            }
            let mut out = head.clone();
            for (k, (nf, nl, mut syms)) in groups {
                syms.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
                let mut seen = std::collections::HashSet::new();
                syms.retain(|(_, n)| seen.insert(n.clone()));
                let top: Vec<String> = syms.iter().take(3).map(|(u, n)| format!("{n} x{u}")).collect();
                out.push_str(&format!("{k} ({nf} files, {nl}L){}{}\n", if top.is_empty() { "" } else { ": " }, top.join(", ")));
            }
            if ntok(&out) > budget { if best_dirs.is_none() { let mut cut = String::new(); for l in out.lines() { if ntok(&(cut.clone() + l)) > budget { break; } cut.push_str(l); cut.push('\n'); } cut.push_str("# more directories not shown: pass a subdirectory\n"); best_dirs = Some(cut); } break; }
            best_dirs = Some(out);
        }
        return Report { text: best_dirs.unwrap_or(head), raw_tokens, top: vec![] };
    }
    let maxk = cand.iter().map(|c| c.len()).max().unwrap_or(0);
    let mut best = render(0, 0);
    let mut kbest = 0;
    for k in 1..=maxk { let t = render(k, 0); if ntok(&t) > budget { break; } best = t; kbest = k; }
    for sigs in [4usize, 8, 16, 32] { let t = render(kbest, sigs); if ntok(&t) > budget { break; } best = t; }
    Report { text: best, raw_tokens, top: vec![] }
}

fn cap_sig(s: &str) -> String { if s.chars().count() <= 90 { s.to_string() } else { format!("{}…", s.chars().take(90).collect::<String>()) } }
