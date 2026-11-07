//! Where vtok keeps its state (stored outputs, the usage log, session marks): one canonical directory where things are
//! written, plus older locations that are still read. On Windows the canonical one is `%LOCALAPPDATA%\vtok` whoever
//! launched the process. Before, a process with `HOME` (Git Bash, a hook) and one without (an MCP server that a desktop
//! app starts) used different directories, so the server could not read the outputs a hook had stored and `gain` saw
//! only the usage logged by one of them.
use std::ffi::OsString;
use std::path::PathBuf;

/// The directories in order of preference; the first one is written. `env` looks a variable up, `windows` says whether
/// `%LOCALAPPDATA%` is the canonical place.
pub(crate) fn resolve(env: &dyn Fn(&str) -> Option<OsString>, windows: bool) -> Vec<PathBuf> {
    if let Some(p) = env("VTOK_STATE") { return vec![PathBuf::from(p)]; }
    if let Some(x) = env("XDG_STATE_HOME") { return vec![PathBuf::from(x).join("vtok")]; }
    let under_home = |var: &str| env(var).map(|h| PathBuf::from(h).join(".local/state/vtok"));
    let local = if windows { env("LOCALAPPDATA").map(|a| PathBuf::from(a).join("vtok")) } else { None };
    let mut out: Vec<PathBuf> = Vec::new();
    for d in [local, under_home("HOME"), under_home("USERPROFILE")].into_iter().flatten() {
        if !out.contains(&d) { out.push(d); }
    }
    out
}

/// Every directory vtok reads state from; the first is the one it writes.
pub fn state_dirs() -> Vec<PathBuf> { resolve(&|k| std::env::var_os(k), cfg!(windows)) }

/// Where vtok writes state (None when the machine gives no place for it).
pub fn state_dir() -> Option<PathBuf> { state_dirs().into_iter().next() }
