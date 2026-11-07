//! `vtok init --agent NAME`: write the usage block to the instructions file each agent reads. Rules files only; hooks
//! are not installed for other agents, because their hook protocols need an explicit "allow" to apply a rewritten
//! command, which would skip the user's own approval. Agents with a shell hook can call `vtok rewrite COMMAND`.
use std::path::Path;

/// (name, file relative to the project, text a new file starts with)
const AGENTS: &[(&str, &str, &str)] = &[
    ("claude", "CLAUDE.md", ""),
    ("codex", "AGENTS.md", ""),
    ("kimi", "AGENTS.md", ""),
    ("opencode", "AGENTS.md", ""),
    ("gemini", "GEMINI.md", ""),
    ("copilot", ".github/copilot-instructions.md", ""),
    ("windsurf", ".windsurfrules", ""),
    ("cline", ".clinerules", ""),
    ("kilocode", ".kilocode/rules/vtok.md", ""),
    ("cursor", ".cursor/rules/vtok.mdc", "---\ndescription: Token-saving tools (vtok)\nalwaysApply: true\n---\n\n"),
];

pub fn names() -> String { AGENTS.iter().map(|a| a.0).collect::<Vec<_>>().join(", ") }

/// Write or update the vtok block for `agent` under `root`. Ok(Some(path)) when the file changed, Ok(None) when already current.
pub fn install(agent: &str, root: &Path) -> Result<Option<String>, String> {
    let Some(&(_, rel, header)) = AGENTS.iter().find(|a| a.0 == agent) else { return Err(format!("unknown agent {agent:?}; known: {}", names())) };
    let path = root.join(rel);
    let existing = std::fs::read_to_string(&path).unwrap_or_else(|_| header.to_string());
    let merged = crate::init::merge(&existing);
    if merged == existing && path.exists() { return Ok(None); }
    if let Some(d) = path.parent() { std::fs::create_dir_all(d).map_err(|e| format!("{}: {e}", d.display()))?; }
    std::fs::write(&path, merged).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(Some(path.display().to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_once_keeps_content_and_adds_cursor_frontmatter() {
        let d = std::env::temp_dir().join(format!("vtok-agents-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("GEMINI.md"), "# mine\n").unwrap();
        assert!(install("gemini", &d).unwrap().is_some());
        assert!(install("gemini", &d).unwrap().is_none());
        let g = std::fs::read_to_string(d.join("GEMINI.md")).unwrap();
        assert!(g.starts_with("# mine\n") && g.contains("vtok:begin"));
        install("cursor", &d).unwrap();
        let c = std::fs::read_to_string(d.join(".cursor/rules/vtok.mdc")).unwrap();
        assert!(c.starts_with("---\n") && c.contains("alwaysApply: true") && c.contains("vtok:end"));
        assert!(install("nope", &d).is_err());
        std::fs::remove_dir_all(&d).ok();
    }
}
