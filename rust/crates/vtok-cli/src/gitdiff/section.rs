//! `vtok output ID --file PATH [--max-tokens N]`: the full diff section(s) of one file from a stored patch, verbatim
//! (so it can be read in full or applied), for whatever the compact view cut. PATH matches the new or old path,
//! exactly or as a trailing path component (`a.rs` finds `src/a.rs` when that is the only match).
use super::model::Item;
use super::parse::{is_commit, is_file_start, parse};
use vtok_api::ntok;

/// (old path, new path, first line, end line exclusive) of every file section.
fn sections(lines: &[String]) -> Vec<(String, String, usize, usize)> {
    let starts: Vec<usize> = (0..lines.len()).filter(|&i| is_file_start(&lines[i])).collect();
    starts.iter().map(|&s| {
        let e = (s + 1..lines.len()).find(|&i| is_file_start(&lines[i]) || is_commit(&lines[i])).unwrap_or(lines.len());
        let (old, new) = match parse(&lines[s..e]).into_iter().next() {
            Some(Item::File(f)) => (f.old, f.new),
            _ => (String::new(), String::new()),
        };
        (old, new, s, e)
    }).collect()
}

pub fn file_sections(lines: &[String], path: &str) -> Result<String, String> {
    let all = sections(lines);
    let exact: Vec<&(String, String, usize, usize)> = all.iter().filter(|(o, n, _, _)| o == path || n == path).collect();
    let tail = format!("/{path}");
    let hits = if exact.is_empty() { all.iter().filter(|(o, n, _, _)| o.ends_with(&tail) || n.ends_with(&tail)).collect() } else { exact };
    if hits.is_empty() { return Err(format!("no file {path:?} in this diff ({} files)", all.len())); }
    let names: std::collections::BTreeSet<&str> = hits.iter().map(|(_, n, _, _)| n.as_str()).collect();
    if names.len() > 1 { return Err(format!("{path:?} matches several files: {}", names.into_iter().collect::<Vec<_>>().join(", "))); }
    let mut out = String::new();
    for (_, _, s, e) in hits {
        out.push_str(&format!("# lines {}-{}\n", s + 1, e));
        for l in &lines[*s..*e] { out.push_str(l); out.push('\n'); }
    }
    Ok(out)
}

pub fn output_file(args: &[String]) -> i32 {
    let (mut id, mut path, mut max) = (None, None, 8000usize);
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--file" => { i += 1; path = args.get(i).cloned(); }
            "--max-tokens" => { i += 1; max = args.get(i).and_then(|v| v.parse().ok()).unwrap_or(max); }
            a if !a.starts_with("--") && id.is_none() => id = Some(a.to_string()),
            a => { eprintln!("unknown argument {a:?} (with --file only --max-tokens applies)"); return 2; }
        }
        i += 1;
    }
    let (Some(id), Some(path)) = (id, path) else { eprintln!("usage: vtok output ID --file PATH [--max-tokens N]"); return 2 };
    let res = crate::shell::load(&id).and_then(|lines| file_sections(&lines, &path));
    match res {
        Ok(s) if ntok(&s) <= max => { print!("# output {id}: {path}\n{s}"); 0 }
        Ok(s) => {
            let mut used = 0;
            let mut out = String::new();
            for l in s.lines() { used += ntok(l) + 1; if used > max { break; } out.push_str(l); out.push('\n'); }
            print!("# output {id}: {path} (cut at ~{max} tokens; raise --max-tokens or use vtok output {id} --lines A-B)\n{out}");
            0
        }
        Err(e) => { eprintln!("error: {e}"); 1 }
    }
}
