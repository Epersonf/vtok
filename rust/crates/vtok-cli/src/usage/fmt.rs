//! Small number and time formatting shared by the usage and session reports.

/// 1234567 -> "1,234,567".
pub fn num(n: i64) -> String {
    let digits = n.unsigned_abs().to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 { out.push(','); }
        out.push(c);
    }
    if n < 0 { format!("-{out}") } else { out }
}

/// 6474 -> "6.5k", 280 -> "280".
pub fn tok(n: i64) -> String {
    if n.abs() >= 1000 { format!("{:.1}k", n as f64 / 1000.0) } else { n.to_string() }
}

/// A span of seconds in the unit that reads best.
pub fn dur(secs: u64) -> String {
    if secs < 90 { format!("{secs} s") } else if secs < 5400 { format!("{} min", (secs + 30) / 60) } else { format!("{:.1} h", secs as f64 / 3600.0) }
}

/// The first 8 characters of a session id.
pub fn short(id: &str) -> &str { id.char_indices().nth(8).map_or(id, |(i, _)| &id[..i]) }

/// Whole seconds since 1970.
pub fn now_secs() -> u64 { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs()) }
