//! CSV and TSV: the odd row keeps its line number, short tables keep their header, quoting, ragged rows.
use super::csv::split;
use super::view;

/// A transactions table: `n` rows, row `odd` has an absurd amount and a country seen nowhere else.
pub(super) fn csv(n: usize, odd: Option<usize>) -> Vec<String> {
    let mut v = vec!["txn_id,ts,account,merchant,amount,country,status".to_string()];
    for i in 0..n {
        let (amt, cty) = if Some(i) == odd { ("48213.77".to_string(), "XX") } else { (format!("{:.2}", 5.25 + (i * 37 % 290) as f64), ["US", "US", "GB", "DE"][i % 4]) };
        v.push(format!("T{},2026-10-03T00:{:02}:00Z,ACC-{},{},{amt},{cty},settled", 900000 + i, i % 60, 1000 + i % 97, ["Acme", "Blue", "City"][i % 3]));
    }
    v
}

#[test]
fn the_row_that_does_not_fit_survives_with_its_line_number() {
    let v = view(&csv(1800, Some(1137)), Some("csv")).unwrap();
    assert!(v.starts_with("csv: 1800 rows, 7 fields\nL1: txn_id,ts,account"), "{v}");
    assert!(v.contains("L1139: T901137,") && v.contains("48213.77") && v.contains("country=XX (1 of 1800)"), "{v}");
    assert!(v.contains("amount=48213.77 (next highest 294.25)"), "{v}");
    assert!(v.contains("L2: T900000,") && v.contains("L1801: T901799,") && v.contains("rows omitted"), "{v}");
    assert!(v.contains("country: US 899, DE 450, GB 450, XX 1") && v.contains("same in all: status=settled"), "{v}");
    assert!(v.len() < 2500, "{} chars\n{v}", v.len());
}

#[test]
fn a_short_table_still_shows_its_header_and_first_rows() {
    let v = view(&csv(40, None), Some("csv")).unwrap();
    assert!(v.contains("L1: txn_id,ts,account") && v.contains("L2: T900000,") && v.contains("L41: T900039,"), "{v}");
    assert!(v.contains("outliers: none"), "{v}");
}

#[test]
fn ragged_rows_are_odd() {
    let mut v = csv(300, None);
    v[100] = "T1,2026,ACC-1".to_string();
    let o = view(&v, Some("csv")).unwrap();
    assert!(o.contains("L101: T1,2026,ACC-1") && o.contains("3 fields, expected 7"), "{o}");
}

#[test]
fn many_ragged_rows_mean_it_is_not_a_table() {
    let v: Vec<String> = (0..100).map(|i| "a,".repeat(i % 7 + 1)).collect();
    assert!(view(&v, Some("csv")).is_none());
}

#[test]
fn prose_with_commas_is_not_a_table() {
    let prose: Vec<String> = (0..80).map(|i| format!("line {i} of some plain text, with a comma")).collect();
    assert!(view(&prose, None).is_none());
    assert!(view(&prose[..5], None).is_none());
}

#[test]
fn tab_separated_text_is_recognised_without_a_file_name() {
    let tsv: Vec<String> = csv(60, None).iter().map(|l| l.replace(',', "\t")).collect();
    assert!(view(&tsv, None).unwrap().starts_with("tsv: 60 rows"));
}

#[test]
fn quotes_group_and_double_quotes_escape() {
    assert_eq!(split("a,\"b,c\",\"d\"\"e\",f", ','), vec!["a", "b,c", "d\"e", "f"]);
    assert_eq!(split("x;y", ';'), vec!["x", "y"]);
}
