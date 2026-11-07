//! Reading a command line: flags, positional arguments, the language of a file, encode options, files under a path.
use crate::help::{die, usage};
use std::path::{Path, PathBuf};
use vtok_api::*;

pub(crate) const VALUE_FLAGS: [&str; 8] = ["--view", "--hide-literals", "--table", "--literals", "--lang", "--max-lines", "-e", "-o"];

pub(crate) fn flag(args: &[String], name: &str) -> Option<String> {
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1).cloned())
}

pub(crate) fn flags(args: &[String], name: &str) -> Vec<String> {
    args.iter().enumerate().filter(|(_, a)| *a == name).filter_map(|(i, _)| args.get(i + 1).cloned()).collect()
}

pub(crate) fn has(args: &[String], name: &str) -> bool { args.iter().any(|a| a == name) }

pub(crate) fn positional(args: &[String]) -> Vec<String> {
    let mut out = vec![];
    let mut skip = false;
    for a in args {
        if skip { skip = false; continue; }
        if VALUE_FLAGS.contains(&a.as_str()) { skip = true; continue; }
        if a.starts_with('-') && a.len() > 1 { continue; }
        out.push(a.clone());
    }
    out
}

pub(crate) fn lang_for(args: &[String], path: &str) -> &'static dyn Language {
    match flag(args, "--lang") {
        Some(l) => language(&l).unwrap_or_else(|e| die(&format!("error: {e}"), 2)),
        None => language_for_path(path).unwrap_or_else(|e| die(&format!("error: {e}"), 2)),
    }
}

pub(crate) fn options(args: &[String]) -> Options {
    Options {
        alias: Some(!has(args, "--no-alias")),
        view: flag(args, "--view"),
        hide_literals: flag(args, "--hide-literals").map(|n| n.parse().unwrap_or_else(|_| usage())),
        ..Default::default()
    }
}

pub(crate) fn read(path: &str) -> String { std::fs::read_to_string(path).unwrap_or_else(|e| die(&format!("error: cannot read {path}: {e}"), 2)) }

pub(crate) fn collect(paths: &[String], lang: Option<&str>, out: &mut Vec<PathBuf>) {
    for p in paths {
        let p = Path::new(p);
        if p.is_dir() {
            let mut kids: Vec<PathBuf> = std::fs::read_dir(p).unwrap().filter_map(|e| e.ok()).map(|e| e.path()).collect();
            kids.sort();
            collect(&kids.iter().map(|k| k.to_string_lossy().into_owned()).collect::<Vec<_>>(), lang, out);
        } else {
            let s = p.to_string_lossy();
            let ok = match lang { Some(l) => language(l).map_or(false, |l| l.extensions().iter().any(|e| s.ends_with(e))), None => language_for_path(&s).is_ok() };
            if ok && !s.ends_with(".d.ts") { out.push(p.to_path_buf()); }
        }
    }
}
