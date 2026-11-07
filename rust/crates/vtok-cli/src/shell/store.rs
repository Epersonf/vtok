//! Where full outputs live: one text file per run under `<state>/out`, newest 100 for 7 days, ids are 8 hex digits.
use std::path::{Path, PathBuf};

const KEEP_FILES: usize = 100;
const KEEP_SECS: u64 = 7 * 24 * 3600;
const LATEST: &str = "latest";

pub fn state_dir() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("VTOK_STATE") { return Some(PathBuf::from(p)); }
    if let Some(x) = std::env::var_os("XDG_STATE_HOME") { return Some(PathBuf::from(x).join("vtok")); }
    if let Some(h) = std::env::var_os("HOME") { return Some(PathBuf::from(h).join(".local/state/vtok")); }
    // Windows has no HOME by default.
    if let Some(a) = std::env::var_os("LOCALAPPDATA") { return Some(PathBuf::from(a).join("vtok")); }
    std::env::var_os("USERPROFILE").map(|h| PathBuf::from(h).join(".local/state/vtok"))
}

fn out_dir() -> Option<PathBuf> { state_dir().map(|d| d.join("out")) }

pub fn valid_id(id: &str) -> bool { (6..=32).contains(&id.len()) && id.bytes().all(|b| b.is_ascii_hexdigit()) }

fn fnv(s: &[u8]) -> u64 { let mut h = 0xcbf29ce484222325u64; for b in s { h ^= *b as u64; h = h.wrapping_mul(0x100000001b3); } h }

pub(crate) fn store(text: &str) -> Option<String> { store_in(&out_dir()?, text) }

pub(super) fn store_in(dir: &Path, text: &str) -> Option<String> {
    std::fs::create_dir_all(dir).ok()?;
    #[cfg(unix)]
    { use std::os::unix::fs::PermissionsExt; let _ = std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700)); }
    let mut nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).ok()?.as_nanos();
    let (id, path) = loop {
        let id = format!("{:08x}", (fnv(text.as_bytes()) ^ (nanos as u64).rotate_left(17)) & 0xffff_ffff);
        let path = dir.join(format!("{id}.txt"));
        if !path.exists() { break (id, path); }
        nanos += 1; // an old output with the same id would be overwritten
    };
    std::fs::write(&path, text).ok()?;
    #[cfg(unix)]
    { use std::os::unix::fs::PermissionsExt; let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)); }
    let _ = std::fs::write(dir.join(LATEST), &id); // file times can tie on coarse filesystems; the pointer cannot
    gc(dir);
    Some(id)
}

fn gc(dir: &Path) {
    let mut files = listing(dir);
    files.sort_by(|a, b| b.0.cmp(&a.0));
    let now = std::time::SystemTime::now();
    for (i, (t, p)) in files.iter().enumerate() {
        let old = now.duration_since(*t).map_or(false, |d| d.as_secs() > KEEP_SECS);
        if i >= KEEP_FILES || old { let _ = std::fs::remove_file(p); }
    }
}

fn listing(dir: &Path) -> Vec<(std::time::SystemTime, PathBuf)> {
    std::fs::read_dir(dir).into_iter().flatten().flatten()
        .filter(|e| e.path().extension().map_or(false, |x| x == "txt"))
        .filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path()))).collect()
}

/// The id of the most recently stored output (`vtok output` without an id reads it).
pub(super) fn latest_in(dir: &Path) -> Option<String> {
    if let Ok(id) = std::fs::read_to_string(dir.join(LATEST)) {
        let id = id.trim();
        if valid_id(id) && dir.join(format!("{id}.txt")).exists() { return Some(id.to_string()); }
    }
    let newest = listing(dir).into_iter().max_by(|a, b| a.0.cmp(&b.0))?;
    newest.1.file_stem().and_then(|s| s.to_str()).filter(|s| valid_id(s)).map(String::from)
}

pub(super) fn latest() -> Option<String> { latest_in(&out_dir()?) }

pub fn load(id: &str) -> Result<Vec<String>, String> { load_in(&out_dir().ok_or("no state directory (set HOME or VTOK_STATE)")?, id) }

pub(super) fn load_in(dir: &Path, id: &str) -> Result<Vec<String>, String> {
    if !valid_id(id) { return Err(format!("invalid output id {id:?}")); }
    let text = std::fs::read_to_string(dir.join(format!("{id}.txt"))).map_err(|_| format!("no stored output {id:?} (expired or never stored)"))?;
    Ok(text.split('\n').map(String::from).collect::<Vec<_>>().into_iter().rev().skip_while(|l| l.is_empty()).collect::<Vec<_>>().into_iter().rev().collect())
}
