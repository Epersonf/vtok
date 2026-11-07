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

/// Epoch seconds as "2026-11-07 18:06" in UTC (there is no time-zone database here, so reports say UTC).
pub fn utc(secs: u64) -> String {
    let (y, m, d) = civil((secs / 86_400) as i64);
    let rem = secs % 86_400;
    format!("{y:04}-{m:02}-{d:02} {:02}:{:02}", rem / 3600, rem % 3600 / 60)
}

/// Days since 1970-01-01 to (year, month, day) in the proleptic Gregorian calendar (Howard Hinnant's algorithm).
fn civil(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
    (yoe + era * 400 + i64::from(month <= 2), month, day)
}
