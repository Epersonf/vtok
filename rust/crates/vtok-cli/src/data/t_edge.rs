//! Awkward inputs: wide tables, Windows line ends, a record that is odd in many ways at once, unicode.
use super::t_csv::csv;
use super::view;
use crate::shell::preview;

#[test]
fn wide_tables_end_in_a_count_of_the_fields_left_out() {
    let mut rows = vec![(0..400).map(|c| format!("col{c:03}")).collect::<Vec<_>>().join(",")];
    for r in 0..40 { rows.push((0..400).map(|c| format!("{}", (r * 7 + c * 13) % 50)).collect::<Vec<_>>().join(",")); }
    let v = view(&rows, Some("csv")).unwrap();
    assert!(v.starts_with("csv: 40 rows, 400 fields"), "{}", &v[..80]);
    assert!(v.contains("more fields") && v.len() < 6000, "{} chars", v.len());
}

#[test]
fn windows_line_ends_do_not_leak_into_the_view() {
    let o = preview("cat data.csv", &csv(600, Some(300)).join("\r\n"), 0, 700);
    assert!(!o.contains('\r') && o.contains("48213.77") && o.contains("L302: T900300,"), "{o:?}");
}

#[test]
fn a_record_odd_in_many_ways_lists_four_reasons_and_a_count() {
    let mut rows = vec![(1..=10).map(|c| format!("c{c}")).collect::<Vec<_>>().join(",")];
    for r in 0..300 { rows.push((1..=10).map(|c| if r == 150 { "weird".to_string() } else { format!("ok{}", c + r % 2) }).collect::<Vec<_>>().join(",")); }
    let v = view(&rows, Some("csv")).unwrap();
    let line = v.lines().find(|l| l.starts_with("L152:") && l.contains("<-")).unwrap_or_default();
    assert!(line.matches("(1 of 300)").count() == 4 && line.contains("+6 more"), "{v}");
}

#[test]
fn fields_few_records_have_get_six_lines_and_a_count() {
    // three shared fields, and each record also has one of twelve keys that a map spread into columns would give
    let rows: Vec<_> = (0..40).map(|i| serde_json::json!({"id": i, "state": "ok", "ms": 20 + i % 5, format!("dep{:02}", i % 12): "^1"})).collect();
    let v = view(&[serde_json::to_string(&rows).unwrap()], Some("json")).unwrap();
    let sparse = v.lines().filter(|l| l.trim_start().starts_with("dep")).count();
    assert!(sparse == 6 && v.contains("+6 more fields"), "{v}");
}

#[test]
fn non_ascii_text_is_clipped_on_character_boundaries() {
    let mut rows = vec!["nome,cidade,valor".to_string()];
    for i in 0..60 { rows.push(format!("José {i},São Paulo,{}", 10 + i % 7)); }
    rows[30] = format!("{},Zürich-{},9999", "Ångström".repeat(10), "ü".repeat(60));
    let v = view(&rows, Some("csv")).unwrap();
    assert!(v.contains("São Paulo") && v.contains("L31:") && v.contains('…'), "{v}");
}
