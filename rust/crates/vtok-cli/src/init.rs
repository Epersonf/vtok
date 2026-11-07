//! `vtok init`: the usage note for agents, as a marker-fenced block for any instructions file (AGENTS.md and the like).
//! The MCP server already sends the same guidance in its `instructions` field; this is the fallback for clients that
//! ignore that field. Agent-neutral on purpose.
const BEGIN: &str = "<!-- vtok:begin -->";
const END: &str = "<!-- vtok:end -->";

pub const BLOCK: &str = "## Token-saving tools (Vtok)
- Source files (Python, TypeScript/JavaScript, Rust): to locate code by topic call `vtok_find` with the words of your question (`show=1` returns the best match's code in the same call); `vtok_refs` shows where a name is defined and used. In a known file call `vtok_outline` for names and line ranges, then `vtok_chunk` with the symbol name, instead of reading the whole file.
- Change code with `vtok_edit` (send the edited Vtok of the chunk or symbol; it needs the `rev` from the read). Comments inside a replaced item are lost, so use your normal edit tool for one-line tweaks.
- Commands with long output (tests, builds, installs, listings): use `vtok_run` if it is available, otherwise run `vtok run -- <command>` in the shell. You get errors, a summary and an id; `vtok_output` (or `vtok output ID --grep TEXT`) returns any part of the full output.
";

pub fn fenced() -> String { format!("{BEGIN}\n{BLOCK}{END}\n") }

/// Insert or replace the fenced block in `existing`, leaving everything else untouched.
pub fn merge(existing: &str) -> String {
    if let (Some(a), Some(b)) = (existing.find(BEGIN), existing.find(END)) {
        if a < b {
            let mut out = String::from(&existing[..a]);
            out.push_str(&fenced());
            let rest = &existing[b + END.len()..];
            out.push_str(rest.strip_prefix('\n').unwrap_or(rest));
            return out;
        }
    }
    let mut out = existing.to_string();
    if !out.is_empty() && !out.ends_with('\n') { out.push('\n'); }
    if !out.is_empty() { out.push('\n'); }
    out.push_str(&fenced());
    out
}

pub fn run(args: &[String]) -> i32 {
    match args.iter().position(|a| a == "--write") {
        None => { print!("{}", fenced()); 0 }
        Some(i) => {
            let Some(path) = args.get(i + 1) else { eprintln!("usage: vtok init [--write FILE]   (prints the block; --write adds or updates it in FILE)"); return 2 };
            let existing = std::fs::read_to_string(path).unwrap_or_default();
            let merged = merge(&existing);
            if merged == existing { println!("{path}: already up to date"); return 0; }
            match std::fs::write(path, merged) { Ok(_) => { println!("{path}: vtok block written"); 0 } Err(e) => { eprintln!("{path}: {e}"); 1 } }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appends_then_replaces_in_place() {
        let first = merge("# Notes\nkeep me\n");
        assert!(first.starts_with("# Notes\nkeep me\n\n<!-- vtok:begin -->"));
        assert_eq!(merge(&first), first, "idempotent");
        let edited = first.replace("Token-saving", "OLD");
        let again = merge(&(edited + "tail line\n"));
        assert!(again.contains("Token-saving") && !again.contains("OLD") && again.ends_with("tail line\n"));
        assert_eq!(again.matches(BEGIN).count(), 1);
    }
}
