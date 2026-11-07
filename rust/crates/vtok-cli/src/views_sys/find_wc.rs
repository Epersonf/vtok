//! find (grouped by directory) and wc.
use super::util::*;
use std::collections::BTreeMap;

pub fn find(lines: &[String]) -> Option<String> {
    if lines.len() < 8 || lines.iter().any(|l| l.contains(": ") && l.starts_with("find:")) { return None; }
    let mut groups: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for p in lines.iter().map(|l| l.trim().replace('\\', "/")).filter(|l| !l.is_empty()) {
        let (dir, name) = p.rsplit_once('/').map(|(d, n)| (d.to_string(), n.to_string())).unwrap_or((".".into(), p.clone()));
        groups.entry(dir).or_default().push(name);
    }
    let mut o = format!("{} paths in {} dirs\n", lines.len(), groups.len());
    for (i, (d, names)) in groups.iter().enumerate() {
        if i >= 40 { o.push_str(&format!("... +{} dirs\n", groups.len() - 40)); break; }
        o.push_str(&format!("{d}/ ({}): {}\n", names.len(), joined(names, 12)));
    }
    Some(o)
}

pub fn wc(lines: &[String]) -> Option<String> {
    let rows: Vec<Vec<&str>> = lines.iter().map(|l| l.split_whitespace().collect()).collect();
    if rows.len() < 3 { return None; }
    let n = rows[0].iter().take_while(|t| t.chars().all(|c| c.is_ascii_digit())).count();
    if n == 0 || n > 3 || rows.iter().any(|r| r.len() < n || r[..n].iter().any(|t| !t.chars().all(|c| c.is_ascii_digit()))) { return None; }
    let name_of = |r: &Vec<&str>| r[n..].join(" ");
    let names: Vec<String> = rows.iter().map(name_of).filter(|x| !x.is_empty() && x != "total").collect();
    let prefix = names.first().and_then(|x| x.rsplit_once('/').map(|(d, _)| format!("{d}/"))).filter(|p| names.iter().all(|x| x.starts_with(p.as_str()))).unwrap_or_default();
    let labels: &[&str] = if n == 3 { &["L", "W", "B"] } else { &["", ""] };
    let mut o = if prefix.is_empty() { String::new() } else { format!("in {prefix}\n") };
    for r in &rows {
        let cols: Vec<String> = r[..n].iter().enumerate().map(|(i, c)| format!("{c}{}", labels[i])).collect();
        let name = name_of(r);
        o.push_str(format!("{} {}\n", cols.join(" "), name.trim_start_matches(prefix.as_str())).replace(" \n", "\n").as_str());
    }
    Some(o)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn l(s: &str) -> Vec<String> { s.lines().map(String::from).collect() }
    #[test]
    fn find_groups_by_dir() {
        let mut raw = String::new();
        for i in 0..20 { raw.push_str(&format!("./src/mod_with_long_name/file_number_{i}.rs\n")); }
        raw.push_str("./README.md\n");
        let lines = l(&raw);
        let o = find(&lines).unwrap();
        assert!(o.starts_with("21 paths in 2 dirs") && o.contains("./src/mod_with_long_name/ (20): file_number_0.rs") && o.contains("+8 more"));
        assert!(o.len() < raw.len());
    }
    #[test]
    fn wc_strips_prefix() {
        let raw = "  10  40  300 src/a.rs\n  20  80  600 src/b.rs\n  30 120  900 total";
        let o = wc(&l(raw)).unwrap();
        assert!(o.contains("in src/") && o.contains("10L 40W 300B a.rs") && o.contains("total"));
        assert!(!o.contains("  "));
    }
}
