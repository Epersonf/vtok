//! `vtok init`: the usage note for agents, as a marker-fenced block for any instructions file (AGENTS.md and the like).
//! The MCP server already sends the same guidance in its `instructions` field; this is the fallback for clients that
//! ignore that field. Agent-neutral on purpose. `--terse` adds the answer-style note (crate::terse) to the block.
const BEGIN: &str = "<!-- vtok:begin -->";
const END: &str = "<!-- vtok:end -->";

pub const BLOCK: &str = "## Token-saving tools (Vtok)
- Source files (Python, TypeScript/JavaScript, Rust, Java, Go, C, C++, C#, Dart, CSS, HTML, SQL): to locate code by topic call `vtok_find` with the words of your question (`show=1` returns the best match's code in the same call); `vtok_refs` shows where a name is defined and used. In a known file call `vtok_outline` for names and line ranges, then `vtok_chunk` with the symbol name, instead of reading the whole file.
- Change code with `vtok_edit` (send the edited Vtok of the chunk or symbol; it needs the `rev` from the read). Comments inside a replaced item are lost; for a one-line tweak, or a file with no Vtok notation, send `replace` (a list of `{old, new}` exact texts) instead. The read tools return the plain text themselves when Vtok would save little, so there is nothing to decide beforehand.
- Commands with long output (tests, builds, installs, listings): use `vtok_run` if it is available, otherwise run `vtok run -- <command>` in the shell. You get the errors and a summary. The full output is stored: if the view cut something or the command failed, its last line names an id, and `vtok_output` (or `vtok output [ID] --grep TEXT`) returns any part of it; without an id it reads the most recent run (for example the warnings behind a count like `4 warnings`).
";

/// The fenced block; `extra` (the terse note, or "") goes after the tool guidance, inside the markers.
pub fn fenced(extra: &str) -> String {
    let sep = if extra.is_empty() { "" } else { "\n" };
    format!("{BEGIN}\n{BLOCK}{sep}{extra}{END}\n")
}

/// Insert or replace the fenced block in `existing`, leaving everything else untouched.
pub fn merge(existing: &str, extra: &str) -> String {
    if let (Some(a), Some(b)) = (existing.find(BEGIN), existing.find(END)) {
        if a < b {
            let mut out = String::from(&existing[..a]);
            out.push_str(&fenced(extra));
            let rest = &existing[b + END.len()..];
            out.push_str(rest.strip_prefix('\n').unwrap_or(rest));
            return out;
        }
    }
    let mut out = existing.to_string();
    if !out.is_empty() && !out.ends_with('\n') { out.push('\n'); }
    if !out.is_empty() { out.push('\n'); }
    out.push_str(&fenced(extra));
    out
}

/// `--terse` with an optional variant after it: Ok("") when absent, Ok(note) when present, Err on an unknown variant.
fn terse_arg(args: &[String]) -> Result<&'static str, String> {
    let Some(i) = args.iter().position(|a| a == "--terse") else { return Ok("") };
    let variant = args.get(i + 1).map(String::as_str).filter(|v| !v.starts_with("--"));
    crate::terse::note(variant).ok_or_else(|| format!("unknown --terse variant {:?}; known: {}", variant.unwrap_or(""), crate::terse::VARIANTS))
}

pub fn run(args: &[String]) -> i32 {
    let extra = match terse_arg(args) { Ok(t) => t, Err(e) => { eprintln!("{e}"); return 2 } };
    if let Some(k) = args.iter().position(|a| a == "--hook") {
        let Some(name) = args.get(k + 1) else { eprintln!("usage: vtok init --hook AGENT [--global]   (one of: {})", crate::hook_install::NAMES); return 2 };
        return match crate::hook_install::install(name, args.iter().any(|a| a == "--global")) { Ok(m) => { println!("{m}"); 0 } Err(e) => { eprintln!("{e}"); 2 } };
    }
    if let Some(i) = args.iter().position(|a| a == "--agent") {
        let Some(name) = args.get(i + 1) else { eprintln!("usage: vtok init --agent NAME [--terse]   (one of: {})", crate::agents::names()); return 2 };
        return match crate::agents::install(name, std::path::Path::new("."), extra) { Ok(Some(p)) => { println!("{p}: vtok block written"); 0 } Ok(None) => { println!("{name}: already up to date"); 0 } Err(e) => { eprintln!("{e}"); 2 } };
    }
    if let Some(i) = args.iter().position(|a| a == "--claude-hook") {
        let Some(path) = args.get(i + 1) else { eprintln!("usage: vtok init --claude-hook SETTINGS.json   (e.g. ~/.claude/settings.json or .claude/settings.json)"); return 2 };
        return match crate::hook::install(path) { Ok(true) => { println!("{path}: vtok hook added (long shell output is wrapped in `vtok run`)"); 0 } Ok(false) => { println!("{path}: already up to date"); 0 } Err(e) => { eprintln!("{e}"); 1 } };
    }
    match args.iter().position(|a| a == "--write") {
        // `--terse` alone prints just the note, for pasting anywhere.
        None if !extra.is_empty() => { print!("{extra}"); 0 }
        None => { print!("{}", fenced("")); 0 }
        Some(i) => {
            let Some(path) = args.get(i + 1) else { eprintln!("usage: vtok init [--write FILE] [--terse]   (prints the block; --write adds or updates it in FILE)"); return 2 };
            let existing = std::fs::read_to_string(path).unwrap_or_default();
            let merged = merge(&existing, extra);
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
        let first = merge("# Notes\nkeep me\n", "");
        assert!(first.starts_with("# Notes\nkeep me\n\n<!-- vtok:begin -->"));
        assert_eq!(merge(&first, ""), first, "idempotent");
        let edited = first.replace("Token-saving", "OLD");
        let again = merge(&(edited + "tail line\n"), "");
        assert!(again.contains("Token-saving") && !again.contains("OLD") && again.ends_with("tail line\n"));
        assert_eq!(again.matches(BEGIN).count(), 1);
    }

    #[test]
    fn terse_goes_inside_the_block_and_can_be_removed_again() {
        let with = merge("# mine\n", crate::terse::NOTE);
        let (a, b) = (with.find(BEGIN).unwrap(), with.find(END).unwrap());
        assert!(with[a..b].contains(crate::terse::NOTE.trim_end()));
        assert_eq!(merge(&with, crate::terse::NOTE), with, "idempotent with the note");
        let without = merge(&with, "");
        assert!(!without.contains(crate::terse::NOTE.lines().next().unwrap()) && without.starts_with("# mine\n"));
    }

    #[test]
    fn terse_flag_parsing() {
        let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        assert_eq!(terse_arg(&s(&["--write", "AGENTS.md"])), Ok(""));
        assert_eq!(terse_arg(&s(&["--terse", "--write", "AGENTS.md"])), Ok(crate::terse::NOTE));
        assert_eq!(terse_arg(&s(&["--agent", "codex", "--terse"])), Ok(crate::terse::NOTE));
        assert!(terse_arg(&s(&["--terse", "loud"])).is_err());
    }
}
