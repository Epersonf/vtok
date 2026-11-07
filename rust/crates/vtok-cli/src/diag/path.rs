//! Paths in diagnostics: `/` separators, and relative to the directory the command ran in when they lie below it
//! (eslint, phpstan and others print absolute paths, which would repeat in every place list).
use std::path::Path;
use std::sync::Mutex;

static BASE: Mutex<Option<String>> = Mutex::new(None);

fn norm(p: &Path) -> String { p.to_string_lossy().replace('\\', "/").trim_start_matches("//?/").trim_end_matches('/').to_string() }

/// Remember where the command runs (`None`: the current directory). Called before it is spawned.
pub fn set_base(cwd: Option<&str>) {
    let cur = std::env::current_dir().ok();
    let dir = match (cwd, cur) { (Some(d), Some(c)) => Some(c.join(d)), (Some(d), None) => Some(d.into()), (None, c) => c };
    if let Ok(mut b) = BASE.lock() { *b = dir.map(|d| norm(&d)); }
}

fn base() -> String {
    match BASE.lock().ok().and_then(|b| b.clone()) {
        Some(b) => b,
        None => std::env::current_dir().map(|d| norm(&d)).unwrap_or_default(),
    }
}

/// `path` with `/` separators, without the `base` directory in front of it. Windows drive letters compare ignoring case.
pub(super) fn relative_to(path: &str, base: &str) -> String {
    let p = path.replace('\\', "/");
    match p.get(..base.len()) {
        Some(head) if !base.is_empty() && p.len() > base.len() + 1 && p.as_bytes()[base.len()] == b'/' && head.eq_ignore_ascii_case(base) => p[base.len() + 1..].to_string(),
        _ => p,
    }
}

pub(super) fn relative(path: &str) -> String { relative_to(path, &base()) }
