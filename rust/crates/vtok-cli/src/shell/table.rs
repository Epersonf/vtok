//! Column-aligned tables: the padding is pure cost. Collapse inner runs of 3+ spaces and drop `---- ----` rulers.
//! Only for text that no per-tool view will parse: those views rely on the padding to find their columns.

pub(super) fn squeeze_table(lines: &[String]) -> Option<Vec<String>> {
    let rows = lines.iter().filter(|l| !l.trim().is_empty()).count();
    let padded = |l: &str| l.trim_start().contains("   ");
    if rows < 5 || lines.iter().filter(|l| padded(l)).count() * 10 < rows * 6 { return None; }
    let sq = |l: &str| -> String {
        let t = l.trim_start();
        let lead = &l[..l.len() - t.len()];
        let mut out = String::from(lead);
        let mut run = 0;
        for c in t.chars() {
            if c == ' ' { run += 1; continue; }
            if run > 0 { if run >= 3 { out.push(' '); } else { out.push_str(&" ".repeat(run)); } run = 0; }
            out.push(c);
        }
        out
    };
    Some(lines.iter().filter(|l| !(l.len() > 3 && l.chars().all(|c| matches!(c, '-' | '=' | ' ')))).map(|l| sq(l)).collect())
}
