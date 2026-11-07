//! The hook also wraps a bare `cat FILE` of a large data file (CSV, TSV, JSON, NDJSON, YAML): the shell tool cuts such a
//! file in the middle anyway, and `vtok run` shows it as a table with the odd records. Only a plain `cat` of one existing
//! file of 30,000 bytes or more qualifies. `risk::low_risk` does not list `cat`, so the agents whose hook protocol turns a
//! rewrite into an "allow" (Cursor, Codex, Gemini CLI, Antigravity) never get this rewrite.
use std::path::{Path, PathBuf};

/// The shell tool's default output limit: a file this large is cut by the tool itself.
pub(super) const BIG: u64 = 30_000;

/// A file name as the shell wrote it, on this system (Git Bash writes `C:\x` as `/c/x`).
fn native(name: &str) -> PathBuf {
    let b = name.as_bytes();
    if cfg!(windows) && b.len() > 2 && b[0] == b'/' && b[1].is_ascii_alphabetic() && b[2] == b'/' {
        return PathBuf::from(format!("{}:{}", b[1].to_ascii_uppercase() as char, &name[2..]));
    }
    PathBuf::from(name)
}

fn plain(word: &str) -> &str { word.trim_matches(['"', '\'']) }

/// The one file a plain `cat` prints, with its size in bytes: `args` (what follows `cat`) is a single name with no flag, no
/// `~` and no `$`, and the file exists. Relative names are read from `base`, the directory the shell is in when it is known.
pub(super) fn sole_file(args: &[&str], base: Option<&Path>) -> Option<(String, u64)> {
    let [name] = args else { return None };
    let name = plain(name);
    if name.starts_with(['-', '~']) || name.contains('$') { return None; }
    let p = native(name);
    let path = if p.is_absolute() { Some(p) } else { base.map(|b| b.join(p)) }?;
    std::fs::metadata(path).ok().filter(|m| m.is_file()).map(|m| (name.to_string(), m.len()))
}

/// True when `args` (what follows `cat`) is one data file of at least `BIG` bytes.
pub fn big_data_file(args: &[&str], base: Option<&Path>) -> bool {
    sole_file(args, base).is_some_and(|(name, len)| len >= BIG && name.rsplit_once('.').is_some_and(|(_, ext)| crate::data::is_data_ext(ext)))
}

/// Where the shell is after `seg`: moved by a plain `cd DIR`, unchanged by anything else, unknown (None) after a `cd`
/// that cannot be followed (`cd -`, `cd ~`, a variable).
pub fn after_cd(base: Option<PathBuf>, seg: &str) -> Option<PathBuf> {
    let w: Vec<&str> = seg.split_whitespace().collect();
    if w.first() != Some(&"cd") { return base; }
    match w.as_slice() {
        [_, dir] if !dir.starts_with(['-', '~']) && !dir.contains('$') => {
            let p = native(plain(dir));
            if p.is_absolute() { Some(p) } else { Some(base?.join(p)) }
        }
        _ => None,
    }
}
