//! `vtok outline | symbols | split | chunk | apply`: one file seen as symbols and chunks.
use crate::args::{flag, flags, has, lang_for, positional, read};
use crate::config;
use crate::help::usage;
use std::collections::BTreeMap;
use std::path::Path;
use vtok_api::*;

pub(crate) fn outline_cmd(rest: &[String]) -> i32 {
    let pos = positional(rest);
    let file = pos.first().unwrap_or_else(|| usage());
    let l = lang_for(rest, file);
    match outline(l.name(), &read(file)) {
        Ok(s) => { print!("{s}"); 0 }
        Err(e) => { eprintln!("error: {e}"); 1 }
    }
}

pub(crate) fn symbols_cmd(rest: &[String]) -> i32 {
    let pos = positional(rest);
    let file = pos.first().unwrap_or_else(|| usage());
    let l = lang_for(rest, file);
    match symbols(l.name(), &read(file)) {
        Ok(v) => { for (id, a, b) in v { println!("{id}\t{a}\t{b}"); } 0 }
        Err(e) => { eprintln!("error: {e}"); 1 }
    }
}

pub(crate) fn split_cmd(cmd: &str, rest: &[String]) -> i32 {
    let pos = positional(rest);
    let file = pos.first().unwrap_or_else(|| usage());
    let l = lang_for(rest, file);
    let max_lines: usize = flag(rest, "--max-lines").map(|n| n.parse().unwrap_or_else(|_| usage())).unwrap_or_else(|| config::max_lines(Some(Path::new(file))).0);
    let src = read(file);
    let res = (|| -> Result<i32> {
        match cmd {
            "split" => {
                let s = split_file(l.name(), &src, max_lines)?;
                let raw = ntok(&normalize_newlines(&src));
                print!("{}", s.index);
                if s.whole_file {
                    eprintln!("# {} lines <= {max_lines}: not split (c1 = whole file)", src.lines().count());
                } else {
                    if !s.table.is_empty() { eprintln!("# alias table\n{}", render_table(&s.table)); }
                    let n = ntok(&s.index);
                    eprintln!("# raw={raw} tokens; index={n} ({}%); {} chunks", 100 * n / raw.max(1), s.chunks.len());
                }
            }
            "chunk" => {
                let id = pos.get(1).unwrap_or_else(|| usage());
                print!("{}", chunk_of(l.name(), &src, id, max_lines)?);
            }
            _ => {
                let mut edits: BTreeMap<String, String> = BTreeMap::new();
                for spec in flags(rest, "-e") {
                    let (id, path) = spec.split_once('=').unwrap_or_else(|| usage());
                    edits.insert(id.to_string(), read(path));
                }
                let out = apply_chunks(l.name(), &src, &edits, max_lines, has(rest, "--force"))?;
                match flag(rest, "-o") { Some(o) => std::fs::write(o, out).unwrap(), None => print!("{out}") }
            }
        }
        Ok(0)
    })();
    match res { Ok(c) => c, Err(e) => { eprintln!("error: {e}"); 1 } }
}
