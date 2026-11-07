//! The `vtok discover` report: what the hook reaches, what `vtok run` could reach, and what the hook never sees.
use super::scan::Scan;
use std::collections::HashMap;

fn pct(part: u64, whole: u64) -> f64 {
    100.0 * part as f64 / whole.max(1) as f64
}

/// "0.6%" below ten percent, "93%" above: a small share must not round up to a whole point.
fn share(part: u64, whole: u64) -> String {
    let p = pct(part, whole);
    if p < 10.0 { format!("{p:.1}%") } else { format!("{p:.0}%") }
}

#[derive(Default)]
struct Row {
    calls: u64,
    hooked: u64,
    wide: u64, // calls only the `hook-wide` rule set would wrap
    raw: u64,
    shown: u64,
}

pub fn print(s: &Scan, min_tokens: usize) {
    let (mut big_raw, mut all_saved, mut hook_raw, mut hook_saved, mut hook_calls) = (0u64, 0u64, 0u64, 0u64, 0u64);
    let (mut wide_calls, mut wide_raw, mut wide_saved) = (0u64, 0u64, 0u64);
    let mut rows: HashMap<&str, Row> = HashMap::new();
    for c in &s.big {
        big_raw += c.raw;
        all_saved += c.raw - c.shown;
        if c.hooked {
            hook_calls += 1;
            hook_raw += c.raw;
            hook_saved += c.raw - c.shown;
        } else if c.wide {
            wide_calls += 1;
            wide_raw += c.raw;
            wide_saved += c.raw - c.shown;
        }
        let r = rows.entry(&c.key).or_default();
        r.calls += 1;
        r.hooked += c.hooked as u64;
        r.wide += (c.wide && !c.hooked) as u64;
        r.raw += c.raw;
        r.shown += c.shown;
    }
    println!("{} transcript file(s), {} shell calls (each counted once), {} tokens of shell output in total", s.files, s.shell_calls, s.shell_raw);
    println!("{} calls had outputs of {min_tokens}+ tokens ({big_raw} tokens)", s.big.len());
    println!("  the hook wraps {hook_calls} of them ({hook_raw} tokens, {} of those) and would save {hook_saved} tokens", share(hook_raw, big_raw));
    if wide_calls > 0 {
        println!("  with `vtok config set hook-wide on` it would also wrap {wide_calls} more ({wide_raw} tokens, {} of those) and save {wide_saved} more tokens", share(wide_raw, big_raw));
    }
    println!("  `vtok run` on every one would save {all_saved} ({}): an upper bound, it also counts cat, scripts and pipes that the hook leaves alone because a cut may hide what the agent needed", share(all_saved, big_raw));

    let mut rows: Vec<(&str, Row)> = rows.into_iter().collect();
    rows.sort_by_key(|(_, r)| std::cmp::Reverse(r.raw - r.shown));
    println!("\n{:<26} {:>5} {:>9} {:>9} {:>7}  hook wraps", "command", "calls", "raw tok", "vtok tok", "saved");
    for (k, r) in rows.iter().take(15) {
        let more = if r.wide > 0 { format!("  +{} with hook-wide", r.wide) } else { String::new() };
        println!("{:<26} {:>5} {:>9} {:>9} {:>7}  {}/{}{more}", k.chars().take(26).collect::<String>(), r.calls, r.raw, r.shown, r.raw - r.shown, r.hooked, r.calls);
    }

    let r = &s.reads;
    let reads = r.prose + r.code + r.data + r.other;
    if r.calls > 0 {
        println!("\nRead results of {min_tokens}+ tokens: {} calls, {reads} tokens: prose {}, code {}, data {}, other {}", r.calls, share(r.prose, reads), share(r.code, reads), share(r.data, reads), share(r.other, reads));
        println!("The shell hook never sees these. vtok_read / vtok_chunk shrink code only; prose has no compact form.");
    }
    println!("\nEstimate: the view `vtok run` gives each command, applied to the output it really returned. A command with a low \"hook wraps\" share can be added with your own filters in ~/.config/vtok/filters.toml (see the README); agents that already pipe through head/tail return smaller outputs than a plain run, which this already reflects.");
}
