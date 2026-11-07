//! Row caps that keep every class. A plain "first N rows" cut loses the rules that happen to come late; this one keeps the
//! first row of each class (rule, code, cop, linter) and fills what room is left with the earliest others, in order.

/// `(class, row)` pairs capped at about `n` rows. The closing line counts what was cut per class:
/// `... +7 more (semi x5, quotes x2)`. A class that has several rows shows at least its first.
pub fn cap_classes(rows: Vec<(String, String)>, n: usize) -> Vec<String> {
    if rows.len() <= n { return rows.into_iter().map(|r| r.1).collect(); }
    let mut seen = std::collections::HashSet::new();
    let mut keep: Vec<bool> = rows.iter().map(|r| seen.insert(r.0.clone())).collect();
    let mut room = n.saturating_sub(keep.iter().filter(|k| **k).count());
    for k in keep.iter_mut().filter(|k| !**k) {
        if room == 0 { break; }
        *k = true;
        room -= 1;
    }
    let mut counts: Vec<(&str, usize)> = vec![];
    for (r, _) in rows.iter().zip(&keep).filter(|(_, k)| !**k) {
        match counts.iter_mut().find(|c| c.0 == r.0) { Some(c) => c.1 += 1, None => counts.push((&r.0, 1)) }
    }
    counts.sort_by(|a, b| b.1.cmp(&a.1));
    let cut: usize = counts.iter().map(|c| c.1).sum();
    let by = counts.iter().filter(|c| !c.0.is_empty()).map(|c| format!("{} x{}", c.0, c.1)).collect::<Vec<_>>().join(", ");
    let by = if by.is_empty() { by } else if by.chars().count() > 300 { format!(" ({}...)", by.chars().take(300).collect::<String>()) } else { format!(" ({by})") };
    let mut out: Vec<String> = rows.into_iter().zip(&keep).filter(|(_, k)| **k).map(|(r, _)| r.1).collect();
    out.push(format!("... +{cut} more{by}"));
    out
}
