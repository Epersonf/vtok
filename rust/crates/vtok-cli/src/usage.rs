//! Local usage log + `vtok gain`: how many tokens did Vtok's tools save, measured against the raw alternative.
//!
//! One JSON line per call in `<state>/usage.jsonl`. Nothing leaves the machine. `raw` is what the same thing would
//! have cost unfiltered (the whole file, the whole command output); for `output` calls it is 0, because reading back
//! part of a stored output is extra cost, not a saving. Counted with tiktoken-compatible o200k (a proxy for the
//! provider's tokenizer). This is an UPPER BOUND on real savings: an agent reading a file might have used grep
//! instead of reading all of it (see bench/nav/RESULTS.md for the realistic comparison).
use serde_json::{json, Value};
use std::io::Write;

const MAX_BYTES: u64 = 5 * 1024 * 1024;

fn path() -> Option<std::path::PathBuf> { crate::shell::state_dir().map(|d| d.join("usage.jsonl")) }

pub fn record(kind: &str, raw: usize, shown: usize, detail: &str) {
    if std::env::var_os("VTOK_NO_USAGE").is_some() { return; }
    let Some(p) = path() else { return };
    let _ = (|| -> std::io::Result<()> {
        if let Some(d) = p.parent() { std::fs::create_dir_all(d)?; }
        if std::fs::metadata(&p).map_or(false, |m| m.len() > MAX_BYTES) {
            let text = std::fs::read_to_string(&p).unwrap_or_default();
            let keep: Vec<&str> = text.lines().collect();
            std::fs::write(&p, keep[keep.len() / 2..].join("\n") + "\n")?;
        }
        let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
        let d: String = detail.chars().take(80).collect();
        let mut f = std::fs::OpenOptions::new().create(true).append(true).open(&p)?;
        writeln!(f, "{}", json!({"t": t, "kind": kind, "raw": raw, "shown": shown, "detail": d}))
    })();
}

pub fn gain_text(days: u64, as_json: bool) -> Result<String, String> {
    let p = path().ok_or("no state directory (set HOME or VTOK_STATE)")?;
    let text = std::fs::read_to_string(&p).unwrap_or_default();
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let since = now.saturating_sub(days * 86400);
    // kind -> (calls, raw, shown)
    let mut by: std::collections::BTreeMap<String, (u64, u64, u64)> = Default::default();
    for l in text.lines() {
        let Ok(v) = serde_json::from_str::<Value>(l) else { continue };
        if v["t"].as_u64().unwrap_or(0) < since { continue; }
        let e = by.entry(v["kind"].as_str().unwrap_or("?").to_string()).or_default();
        e.0 += 1;
        e.1 += v["raw"].as_u64().unwrap_or(0);
        e.2 += v["shown"].as_u64().unwrap_or(0);
    }
    let tot = by.values().fold((0, 0, 0), |a, b| (a.0 + b.0, a.1 + b.1, a.2 + b.2));
    if as_json {
        let rows: Vec<Value> = by.iter().map(|(k, v)| json!({"kind": k, "calls": v.0, "raw": v.1, "shown": v.2})).collect();
        return Ok(json!({"days": days, "rows": rows, "calls": tot.0, "raw": tot.1, "shown": tot.2}).to_string());
    }
    if by.is_empty() { return Ok(format!("no usage recorded in the last {days} days ({})", p.display())); }
    let mut out = format!("{:<8} {:>6} {:>12} {:>12} {:>12} {:>7}\n", "tool", "calls", "raw tokens", "shown", "saved", "saved%");
    let row = |out: &mut String, k: &str, v: (u64, u64, u64)| {
        let saved = v.1 as i64 - v.2 as i64;
        let pct = if v.1 > 0 { 100.0 * saved as f64 / v.1 as f64 } else { 0.0 };
        out.push_str(&format!("{:<8} {:>6} {:>12} {:>12} {:>12} {:>6.1}%\n", k, v.0, v.1, v.2, saved, pct));
    };
    for (k, v) in &by { row(&mut out, k, *v); }
    row(&mut out, "TOTAL", tot);
    out.push_str(&format!("\nlast {days} days. 'raw' = the whole file / whole command output; 'output' reads cost extra (raw 0).\nUpper bound: an agent might have used grep or a partial read instead of the full alternative (see bench/nav/RESULTS.md).\n"));
    Ok(out)
}

/// One short line for a status bar: `vtok saved 12.3k tok (41%) / 7d`. Empty when nothing was recorded.
pub fn line(days: u64) -> String {
    let Ok(j) = gain_text(days, true) else { return String::new() };
    let Ok(v) = serde_json::from_str::<Value>(&j) else { return String::new() };
    let (raw, shown) = (v["raw"].as_i64().unwrap_or(0), v["shown"].as_i64().unwrap_or(0));
    if raw <= 0 { return String::new(); }
    let saved = raw - shown;
    let k = if saved.abs() >= 1000 { format!("{:.1}k", saved as f64 / 1000.0) } else { saved.to_string() };
    format!("vtok saved {k} tok ({:.0}%) / {days}d", 100.0 * saved as f64 / raw as f64)
}

pub fn gain(args: &[String]) -> i32 {
    let days: u64 = args.iter().position(|a| a == "--days").and_then(|i| args.get(i + 1)).and_then(|v| v.parse().ok()).unwrap_or(30);
    if args.iter().any(|a| a == "--line") { println!("{}", line(days)); return 0; }
    match gain_text(days, args.iter().any(|a| a == "--json")) {
        Ok(t) => { println!("{}", t.trim_end()); 0 }
        Err(e) => { eprintln!("{e}"); 1 }
    }
}
