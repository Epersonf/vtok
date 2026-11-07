//! Who gets the token budget. Every file's header line is always paid for. Then, by tier, changed lines are handed
//! out by water-filling: one per-file cap `c` (the largest that fits), so small files show in full and the biggest
//! ones are the first to be cut. Tier 1 = modified hand-written files, 2 = new files (beyond a short preview that is
//! part of the base cost), 3 = deleted files' content. A tier only starts once the one before it is shown in full.
use super::rows::Row;

pub struct Entry { pub header: String, pub rows: Vec<Row>, pub tier: u8, pub floor: usize, pub extra: Option<String>, pub cap: usize, fixed: (usize, usize) }

impl Entry {
    pub fn new(header: String, rows: Vec<Row>, tier: u8, floor: usize, extra: Option<String>) -> Entry {
        let fixed = (vtok_api::ntok(&header) + 1, extra.as_ref().map_or(0, |x| vtok_api::ntok(x) + 1));
        Entry { header, rows, tier, floor, extra, cap: 0, fixed }
    }
    pub fn total(&self) -> usize { self.rows.iter().map(|r| r.changed).sum() }
}

/// Rows shown at cap `c`: (prefix length, changed lines in it). Stops before a header or changed row once `c` is reached.
pub fn take(rows: &[Row], c: usize) -> (usize, usize) {
    let mut shown = 0;
    for (i, r) in rows.iter().enumerate() {
        if (r.head || r.changed > 0) && shown >= c { return (i, shown); }
        if r.changed > 0 && shown + r.changed > c && shown > 0 { return (i, shown); }
        shown += r.changed;
    }
    (rows.len(), shown)
}

fn note_cost(e: &Entry, c: usize) -> usize {
    let t = e.total();
    if c >= t || (e.tier == 3 && c == 0) { 0 } else { 16 + e.fixed.1 }
}

pub fn cost(e: &Entry, c: usize) -> usize {
    let c = c.max(e.floor);
    let (n, _) = take(&e.rows, c);
    e.fixed.0 + e.rows[..n].iter().map(|r| r.cost).sum::<usize>() + if e.rows.is_empty() { 0 } else { note_cost(e, c) }
}

fn spent(es: &[Entry]) -> usize { es.iter().map(|e| cost(e, e.cap)).sum() }

/// Give `tier` the largest uniform cap that keeps the total within `budget`; true when that tier is shown in full.
fn fill(es: &mut [Entry], tier: u8, budget: usize) -> bool {
    let idx: Vec<usize> = (0..es.len()).filter(|&i| es[i].tier == tier).collect();
    let max = idx.iter().map(|&i| es[i].total()).max().unwrap_or(0);
    let try_cap = |es: &mut [Entry], c: usize| -> usize { for &i in &idx { es[i].cap = c.max(es[i].floor); } spent(es) };
    if try_cap(es, max) <= budget { return true; }
    let (mut lo, mut hi) = (0, max);
    while lo < hi {
        let mid = (lo + hi + 1) / 2;
        if try_cap(es, mid) <= budget { lo = mid; } else { hi = mid - 1; }
    }
    try_cap(es, lo);
    // leftover: raise single files one hunk-step further, smallest first, while it still fits
    let mut order = idx.clone();
    order.sort_by_key(|&i| es[i].total());
    for i in order {
        let old = es[i].cap;
        if old >= es[i].total() { continue; }
        es[i].cap = es[i].total().min(old * 2 + 4);
        if spent(es) > budget { es[i].cap = old; }
    }
    false
}

pub fn allocate(es: &mut [Entry], budget: usize) {
    for e in es.iter_mut() { e.cap = e.floor; }
    if spent(es) > budget { for e in es.iter_mut() { e.floor = 0; e.cap = 0; } }
    for tier in 1..=3 { if !fill(es, tier, budget) { return; } }
}
