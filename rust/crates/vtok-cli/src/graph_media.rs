//! Image, video and audio nodes: dimensions (header only), size, and duration when cheaply readable (wav, mp4/mov).
use super::text::svg_size;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

pub(crate) const IMAGE: &[&str] = &["png", "jpg", "jpeg", "gif", "webp", "svg"];
pub(crate) const VIDEO: &[&str] = &["mp4", "mkv", "webm", "mov"];
pub(crate) const AUDIO: &[&str] = &["mp3", "wav"];

/// (width, height) of an image file; SVG is read as text and its text content is returned too.
pub(crate) fn image_info(path: &Path, ext: &str) -> Option<(usize, usize, String)> {
    if ext == "svg" {
        let raw = std::fs::read_to_string(path).ok()?;
        let (w, h) = svg_size(&raw);
        return Some((w, h, super::text::svg_text(&raw)));
    }
    let s = imagesize::size(path).ok()?;
    Some((s.width, s.height, String::new()))
}

fn u32be(b: &[u8]) -> u64 { u32::from_be_bytes([b[0], b[1], b[2], b[3]]) as u64 }

/// Whole seconds of a media file, when the container makes that cheap.
pub(crate) fn duration_secs(path: &Path, ext: &str) -> Option<u64> {
    let mut f = std::fs::File::open(path).ok()?;
    match ext {
        "wav" => {
            let mut h = vec![0u8; 4096];
            let n = f.read(&mut h).ok()?;
            let h = &h[..n];
            let (mut byte_rate, mut p) = (0u64, 12usize);
            while p + 8 <= h.len() {
                let size = u32::from_le_bytes([h[p + 4], h[p + 5], h[p + 6], h[p + 7]]) as u64;
                match &h[p..p + 4] {
                    b"fmt " if p + 20 <= h.len() => byte_rate = u32::from_le_bytes([h[p + 16], h[p + 17], h[p + 18], h[p + 19]]) as u64,
                    b"data" => return if byte_rate > 0 { Some(size / byte_rate) } else { None },
                    _ => {}
                }
                p += 8 + size as usize + (size as usize & 1);
            }
            None
        }
        "mp4" | "mov" => {
            let len = f.metadata().ok()?.len();
            let mut pos = 0u64;
            while pos + 8 <= len {
                f.seek(SeekFrom::Start(pos)).ok()?;
                let mut h = [0u8; 8];
                f.read_exact(&mut h).ok()?;
                let mut size = u32be(&h[..4]);
                if size == 1 { let mut e = [0u8; 8]; f.read_exact(&mut e).ok()?; size = u64::from_be_bytes(e); }
                if &h[4..8] == b"moov" {
                    let mut buf = vec![0u8; size.min(8_000_000) as usize];
                    f.seek(SeekFrom::Start(pos + 8)).ok()?;
                    let n = f.read(&mut buf).ok()?;
                    let i = buf[..n].windows(4).position(|w| w == b"mvhd")? + 4;
                    let b = buf.get(i..i + 32)?;
                    let (ts, dur) = if b[0] == 1 { (u32be(&b[20..24]), u64::from_be_bytes(b[24..32].try_into().ok()?)) } else { (u32be(&b[12..16]), u32be(&b[16..20])) };
                    return if ts > 0 { Some(dur / ts) } else { None };
                }
                if size < 8 { return None; }
                pos += size;
            }
            None
        }
        _ => None,
    }
}
