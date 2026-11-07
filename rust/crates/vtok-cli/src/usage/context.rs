//! Who is calling: the chat's session id (from the agent's environment), where it runs, and whether the call came
//! through the MCP server.
use std::sync::OnceLock;

static VIA: OnceLock<&'static str> = OnceLock::new();

/// `vtok mcp` says so once at start; every other process is "cli".
pub fn set_via(via: &'static str) { let _ = VIA.set(via); }

pub fn via() -> &'static str { VIA.get().copied().unwrap_or("cli") }

/// The id that groups this call with the others of the same chat: `VTOK_SESSION` when set (any agent, any shell), else
/// Claude Code's `CLAUDE_CODE_SESSION_ID`, which its shell tool and its MCP servers inherit.
pub fn session_id() -> Option<String> {
    ["VTOK_SESSION", "CLAUDE_CODE_SESSION_ID"].iter()
        .find_map(|k| std::env::var(k).ok().map(|v| v.trim().to_string()).filter(|v| !v.is_empty()))
}

pub fn cwd() -> String { std::env::current_dir().map(|d| d.to_string_lossy().into_owned()).unwrap_or_default() }

/// A path as a comparable string: `/` separators, no trailing slash, lower case on Windows.
pub fn norm_path(p: &str) -> String {
    let s = p.replace('\\', "/");
    let s = s.trim_end_matches('/');
    if cfg!(windows) { s.to_lowercase() } else { s.to_string() }
}

/// Whether `child` is `parent` or lies inside it (both already through `norm_path`).
pub fn within(child: &str, parent: &str) -> bool { child == parent || child.starts_with(&format!("{parent}/")) }
