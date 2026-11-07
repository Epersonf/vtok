//! psql: aligned tables compacted (padding dropped, cells and rows capped) and expanded records.
use super::util::*;

pub fn view(lines: &[String]) -> Option<String> {
    if lines.iter().any(|l| l.starts_with("-[ RECORD")) { return expanded(lines); }
    let sep = lines.iter().position(|l| l.contains("-+-") || (l.starts_with('-') && l.contains('+')))?;
    if sep == 0 { return None; }
    let cells = |l: &str| l.split('|').map(|c| trunc(c.trim(), 40)).collect::<Vec<_>>().join(" | ");
    let mut o = format!("{}\n", cells(&lines[sep - 1]));
    let rows: Vec<&String> = lines[sep + 1..].iter().filter(|l| l.contains('|')).collect();
    for r in rows.iter().take(30) { o.push_str(&format!("{}\n", cells(r))); }
    if rows.len() > 30 { o.push_str(&format!("... +{} more rows\n", rows.len() - 30)); }
    if let Some(f) = lines.iter().find(|l| l.starts_with('(') && l.ends_with(')')) { o.push_str(&format!("{f}\n")); }
    Some(o)
}

fn expanded(lines: &[String]) -> Option<String> {
    let (mut o, mut n) = (String::new(), 0);
    for l in lines {
        if l.starts_with("-[ RECORD") { n += 1; if n > 10 { continue; } o.push_str(&format!("{}\n", l.trim_matches(|c| c == '-' || c == ' '))); }
        else if n <= 10 && l.contains('|') { let (k, v) = l.split_once('|')?; o.push_str(&format!("  {}={}\n", k.trim(), trunc(v.trim(), 60))); }
    }
    if n > 10 { o.push_str(&format!("... +{} more records\n", n - 10)); }
    Some(o)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn table_is_truncated() {
        let mut raw = vec![" id |     name      |  email".to_string(), "----+---------------+-----------".to_string()];
        for i in 0..50 { raw.push(format!("  {i} | user number {i}  | user{i}@example.com")); }
        raw.push("(50 rows)".to_string());
        let o = view(&raw).unwrap();
        assert!(o.starts_with("id | name | email") && o.contains("+20 more rows") && o.contains("(50 rows)"));
        assert!(!o.contains("---") && !o.contains("user49") && o.len() < raw.join("\n").len());
    }
    #[test]
    fn expanded_records() {
        let mut raw = vec![];
        for i in 1..=12 { raw.push(format!("-[ RECORD {i} ]-----")); raw.push(format!("id   | {i}")); raw.push("name | x".to_string()); }
        let o = view(&raw).unwrap();
        assert!(o.contains("RECORD 10") && !o.contains("RECORD 11") && o.contains("+2 more records") && o.contains("id=3"));
    }
    #[test]
    fn non_table_is_none() { assert!(view(&["INSERT 0 1".to_string()]).is_none()); }
}
