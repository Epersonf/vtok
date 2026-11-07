//! Local usage log and `vtok gain`: how many tokens did vtok's tools save, measured against the alternative.
//!
//! One JSON line per call in `<state>/usage.jsonl` (nothing leaves the machine): the time, the kind of call, the tokens of
//! the alternative (`raw`) and of what vtok returned (`shown`), a short detail and, since the tags were added, the chat's
//! session id, the working directory and whether the call came through the MCP server. `kinds` says how far each `raw` can
//! be trusted, `gain` is the global view, `crate::session` the one per chat or named stretch of work. Tokens are counted
//! with tiktoken-compatible o200k (a proxy for the provider's tokenizer). `VTOK_NO_USAGE=1` turns the log off, and the
//! crate's own unit tests never write it.
//!
//! Files: `context` (who is calling), `rec` (a line), `store` (disk), `kinds` (what a kind means), `filter` (which
//! records), `gain` (the table), `fmt` (numbers and times).
pub(crate) mod context;
pub(crate) mod filter;
pub(crate) mod fmt;
mod gain;
pub(crate) mod kinds;
pub(crate) mod rec;
pub(crate) mod store;
#[cfg(test)] mod t_filter;
#[cfg(test)] mod t_gain;
#[cfg(test)] mod t_store;

pub use context::set_via;
pub use gain::{gain, gain_tool};

/// Log one call. Silent on purpose (a failing log must not break a tool); `health` is how a server finds out.
pub fn record(kind: &str, raw: usize, shown: usize, detail: &str) {
    if cfg!(test) || std::env::var_os("VTOK_NO_USAGE").is_some() { return; }
    let Some(path) = store::log_path() else { return };
    let _ = store::append(&path, &rec::Rec::now(kind, raw, shown, detail));
}

/// Log one `vtok run`: the kind of command (program and subcommand, never its arguments, which may hold secrets), the id of
/// the stored output, and a flag when vtok printed the output as it was (` (tiny)`, ` (not smaller)`).
pub fn record_run(cmd: &str, id: Option<&str>, raw: usize, shown: usize, flag: &str) {
    let kind: String = crate::discover::command_key::key(cmd).chars().take(48).collect();
    record("run", raw, shown, &format!("{kind} #{}{flag}", id.unwrap_or("-")));
}

/// Where records go, or why they cannot: the MCP server says so once when it starts instead of losing them in silence.
pub fn health() -> Result<std::path::PathBuf, String> {
    let path = store::log_path().ok_or("no state directory (set VTOK_STATE)")?;
    if let Some(d) = path.parent() { std::fs::create_dir_all(d).map_err(|e| format!("{}: {e}", d.display()))?; }
    Ok(path)
}
