//! `vtok squeeze FILE`: shrink an instructions / memory file (CLAUDE.md, AGENTS.md, notes) that an agent re-reads every
//! session. Deterministic, no model: trims whitespace, drops decoration and filler phrases from the prose. Code blocks,
//! inline code, URLs, headings, tables and list structure are left exactly as they are.
//!
//! `prose` rewrites a line of prose, `document` walks a whole file, `check` lists the protected items a rewrite lost;
//! `run` here is the `vtok squeeze` command.
use vtok_api::ntok;

mod check;
mod document;
mod prose;
#[cfg(test)] mod t_check;
#[cfg(test)] mod t_document;
#[cfg(test)] mod t_prose;

pub use check::missing;
pub use document::squeeze;

const REWRITE_PROMPT: &str = "Rewrite the file below so an AI agent reads it in fewer tokens with no loss of instructions. Rules: keep every command, path, flag, number, URL, heading and code block exactly as written; delete repetition, examples that restate a rule, and marketing or background prose; use short imperative sentences, no filler. Do not add anything. Save the result to a new file and run `vtok squeeze --check ORIGINAL NEW` to verify nothing protected was lost.\n\n";

pub fn run(args: &[String]) -> i32 {
    let pos: Vec<&String> = args.iter().filter(|a| !a.starts_with("--")).collect();
    if args.iter().any(|a| a == "--check") {
        let (Some(o), Some(n)) = (pos.first(), pos.get(1)) else { eprintln!("usage: vtok squeeze --check ORIGINAL NEW"); return 2 };
        let (Ok(old), Ok(new)) = (std::fs::read_to_string(o), std::fs::read_to_string(n)) else { eprintln!("cannot read one of the files"); return 1 };
        let m = missing(&old, &new);
        let (a, b) = (ntok(&old), ntok(&new));
        println!("{a} -> {b} tokens ({:.0}% less)", 100.0 * (1.0 - b as f64 / a.max(1) as f64));
        if m.is_empty() { println!("ok: every code block, inline code, URL and heading is still there"); return 0; }
        println!("MISSING {} protected item(s):", m.len());
        for x in m.iter().take(20) { println!("  {}", x.lines().next().unwrap_or("")); }
        return 1;
    }
    if args.iter().any(|a| a == "--rewrite") {
        let Some(p) = pos.first() else { eprintln!("usage: vtok squeeze --rewrite FILE   (prints a rewrite request for your agent)"); return 2 };
        let Ok(t) = std::fs::read_to_string(p) else { eprintln!("{p}: cannot read as text"); return 1 };
        print!("{REWRITE_PROMPT}--- {p} ---\n{t}");
        return 0;
    }
    let write = args.iter().any(|a| a == "--write");
    let Some(path) = pos.first() else { eprintln!("usage: vtok squeeze FILE [--write] | --rewrite FILE | --check ORIGINAL NEW"); return 2 };
    let Ok(text) = std::fs::read_to_string(path) else { eprintln!("{path}: cannot read as text"); return 1 };
    let new = squeeze(&text);
    let (a, b) = (ntok(&text), ntok(&new));
    if write {
        if new == text { println!("{path}: nothing to squeeze"); return 0; }
        let bak = format!("{path}.orig");
        if !std::path::Path::new(&bak).exists() { if let Err(e) = std::fs::write(&bak, &text) { eprintln!("{bak}: {e}"); return 1; } }
        if let Err(e) = std::fs::write(path, &new) { eprintln!("{path}: {e}"); return 1; }
        println!("{path}: {a} -> {b} tokens ({:.0}% less); original kept as {bak}", 100.0 * (1.0 - b as f64 / a.max(1) as f64));
    } else {
        print!("{new}");
        eprintln!("# {a} -> {b} tokens ({:.0}% less); nothing written (use --write)", 100.0 * (1.0 - b as f64 / a.max(1) as f64));
    }
    0
}
