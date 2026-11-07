//! The source files of a project (skipping vendored and build directories) and the base name of a symbol id.
use std::path::{Path, PathBuf};
use vtok_api::language_for_path;

const SKIP_DIRS: &[&str] = &["target", "node_modules", "venv", "dist", "build", "__pycache__", "vendor", "site-packages"];
const MAX_FILES: usize = 5000;
const MAX_BYTES: u64 = 1_000_000;

pub(crate) fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    let mut entries: Vec<_> = rd.flatten().collect();
    entries.sort_by_key(|e| e.file_name());
    for e in entries {
        if out.len() >= MAX_FILES { return; }
        let name = e.file_name().to_string_lossy().to_string();
        let path = e.path();
        let Ok(ft) = e.file_type() else { continue };
        if ft.is_dir() {
            if name.starts_with('.') || SKIP_DIRS.contains(&name.as_str()) { continue; }
            walk(&path, out);
        } else if ft.is_file() && language_for_path(&name).is_ok() && e.metadata().map_or(false, |m| m.len() <= MAX_BYTES) {
            out.push(path);
        }
    }
}

pub(crate) fn base(id: &str) -> &str {
    let id = id.split('#').next().unwrap_or(id);
    id.rsplit('.').next().unwrap_or(id)
}
