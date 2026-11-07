//! The usage log on disk: `<state>/usage.jsonl`, one JSON line per call; when it passes 5 MB the newest half is kept.
use super::rec::Rec;
use std::io::Write;
use std::path::{Path, PathBuf};

const MAX_BYTES: u64 = 5 * 1024 * 1024;

/// A log file that was read and the records in it.
pub struct Source { pub path: PathBuf, pub recs: Vec<Rec> }

/// Where records are written (None when the machine gives no state directory).
pub fn log_path() -> Option<PathBuf> { crate::shell::state_dir().map(|d| d.join("usage.jsonl")) }

pub fn append(path: &Path, rec: &Rec) -> std::io::Result<()> {
    if let Some(d) = path.parent() { std::fs::create_dir_all(d)?; }
    if std::fs::metadata(path).map_or(false, |m| m.len() > MAX_BYTES) {
        let text = read_text(path);
        let lines: Vec<&str> = text.lines().collect();
        std::fs::write(path, lines[lines.len() / 2..].join("\n") + "\n")?;
    }
    let mut f = std::fs::OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(f, "{}", rec.to_json())
}

fn read_text(path: &Path) -> String { String::from_utf8_lossy(&std::fs::read(path).unwrap_or_default()).into_owned() }

/// The records of one file; lines that are not records are skipped.
pub fn read_file(path: &Path) -> Vec<Rec> {
    read_text(path).lines().filter_map(|l| Rec::from_json(&serde_json::from_str(l).ok()?)).collect()
}

/// Every usage log vtok can see: the one it writes and the older locations (see `shell::state`).
pub fn read_all() -> Vec<Source> {
    crate::shell::state_dirs().into_iter().map(|d| d.join("usage.jsonl")).filter(|p| p.exists())
        .map(|path| Source { recs: read_file(&path), path }).collect()
}
