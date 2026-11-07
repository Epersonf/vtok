//! `vtok languages | grammar | encode | decode | roundtrip | stats`: the notation itself, file by file.
use crate::args::{collect, flag, lang_for, options, positional, read};
use crate::help::{die, usage};
use crate::jsonmap::{json_map, parse_json_map};
use std::collections::HashMap;
use vtok_api::*;

pub(crate) fn languages_cmd() -> i32 {
    for l in languages() { println!("{:12} {}", l.name(), l.extensions().join(" ")); }
    0
}

pub(crate) fn grammar_cmd(rest: &[String]) -> i32 {
    let l = language(&flag(rest, "--lang").unwrap_or_else(|| usage())).unwrap_or_else(|e| die(&format!("error: {e}"), 2));
    println!("{}", l.grammar_prompt());
    0
}

pub(crate) fn encode_cmd(rest: &[String]) -> i32 {
    let pos = positional(rest);
    let file = pos.first().unwrap_or_else(|| usage());
    let l = lang_for(rest, file);
    let o = options(rest).to_opts().unwrap_or_else(|e| die(&format!("error: {e}"), 2));
    match l.encode(&read(file), &o) {
        Ok(r) => {
            print!("{}", r.vtok);
            if !r.table.is_empty() {
                eprintln!("# alias table\n{}", render_table(&r.table));
                if let Some(t) = flag(rest, "--table") { std::fs::write(t, render_table(&r.table) + "\n").unwrap(); }
            }
            if let (false, Some(p)) = (r.literals.is_empty(), flag(rest, "--literals")) { std::fs::write(p, json_map(&r.literals)).unwrap(); }
            0
        }
        Err(e) => { eprintln!("error: {e}"); 1 }
    }
}

pub(crate) fn decode_cmd(rest: &[String]) -> i32 {
    let pos = positional(rest);
    let file = pos.first().unwrap_or_else(|| usage());
    let l = lang_for(rest, file);
    let table = flag(rest, "--table").map(|t| parse_table(&read(&t))).unwrap_or_default();
    let lits: HashMap<String, String> = flag(rest, "--literals").map(|p| parse_json_map(&read(&p))).unwrap_or_default();
    match l.decode(&read(file), &table, &lits) {
        Ok(s) => { print!("{s}"); 0 }
        Err(e) => { eprintln!("parse error: {e}"); 1 }
    }
}

pub(crate) fn roundtrip_cmd(cmd: &str, rest: &[String]) -> i32 {
    let mut files = vec![];
    let lang_name = flag(rest, "--lang");
    collect(&positional(rest), lang_name.as_deref(), &mut files);
    let o = options(rest).to_opts().unwrap_or_else(|e| die(&format!("error: {e}"), 2));
    let (mut ok, mut fail, mut raw, mut vt) = (0, 0, 0usize, 0usize);
    for f in &files {
        let path = f.to_string_lossy().into_owned();
        let src = std::fs::read_to_string(f).unwrap_or_default();
        match lang_for(rest, &path).encode(&src, &o) {
            Ok(r) => {
                ok += 1;
                if cmd == "stats" { raw += ntok(&src); vt += ntok(&r.vtok) + ntok(&render_table(&r.table)); }
            }
            Err(e) => { fail += 1; eprintln!("FAIL {path}: {e}"); }
        }
    }
    println!("roundtrip ok={ok} fail={fail}");
    if cmd == "stats" { println!("tokens raw={raw} vtok+table={vt} saving={:.1}%", 100.0 * (1.0 - vt as f64 / raw.max(1) as f64)); }
    if fail > 0 { 1 } else { 0 }
}
