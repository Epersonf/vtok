//! What counts as odd: rare values, other shapes, text among numbers, fragmented columns that flag nothing.
use super::t_csv::csv;
use super::view;

/// `csv(n)` with `edit(line index, cells)` applied to every data line.
fn with(n: usize, edit: impl Fn(usize, &mut Vec<String>)) -> Vec<String> {
    csv(n, None).iter().enumerate().map(|(i, l)| {
        if i == 0 { return l.clone(); }
        let mut c: Vec<String> = l.split(',').map(String::from).collect();
        edit(i, &mut c);
        c.join(",")
    }).collect()
}

#[test]
fn an_unremarkable_table_has_no_outliers() {
    assert!(view(&csv(300, None), Some("csv")).unwrap().contains("outliers: none"));
}

#[test]
fn text_in_a_numeric_column_is_odd() {
    let v = with(300, |i, c| if i == 50 { c[4] = "N/A".into() });
    let o = view(&v, Some("csv")).unwrap();
    assert!(o.contains("L51: ") && o.contains("amount=N/A (not a number; the others are)"), "{o}");
}

#[test]
fn an_id_of_another_shape_is_odd() {
    let v = with(300, |i, c| if i == 70 { c[2] = "ZZZ".into() });
    let o = view(&v, Some("csv")).unwrap();
    assert!(o.contains("L71: ") && o.contains("account=ZZZ (unlike the rest, 1 of 300)"), "{o}");
}

#[test]
fn a_blank_cell_in_a_full_column_is_odd() {
    let v = with(300, |i, c| if i == 20 { c[3] = String::new() });
    let o = view(&v, Some("csv")).unwrap();
    assert!(o.contains("L21: ") && o.contains("no merchant (299 of 300 have it)"), "{o}");
}

#[test]
fn a_fragmented_column_has_no_odd_value() {
    // thirty countries, ten rows each: none of them is "the rare one"
    let v = with(300, |i, c| c[5] = format!("{}{}", (b'A' + (i % 30) as u8 / 2) as char, (b'A' + (i % 30) as u8 % 2) as char));
    assert!(view(&v, Some("csv")).unwrap().contains("outliers: none"));
}

#[test]
fn a_heavy_tailed_column_is_not_all_outliers() {
    // every tenth amount is 100x the rest: too many to be exceptions
    let v = with(300, |i, c| if i % 10 == 0 { c[4] = "99999.5".into() });
    assert!(!view(&v, Some("csv")).unwrap().contains("next highest"));
}

#[test]
fn an_id_that_appears_twice_is_odd() {
    let v = with(300, |i, c| if i == 250 { c[0] = "T900010".into() });
    let o = view(&v, Some("csv")).unwrap();
    assert!(o.contains("L12: T900010,") && o.contains("L251: T900010,"), "{o}");
    assert!(o.contains("txn_id=T900010 (appears 2 times in a column that is otherwise unique)"), "{o}");
}

#[test]
fn a_timestamp_out_of_order_is_odd() {
    // timestamps rise by the minute; row 120 goes back a day
    let v = with(300, |i, c| c[1] = if i == 120 { "2026-10-02T00:00:00Z".into() } else { format!("2026-10-03T{:02}:{:02}:00Z", i / 60 + 1, i % 60) });
    let o = view(&v, Some("csv")).unwrap();
    assert!(o.contains("L121: ") && o.contains("ts=2026-10-02T00:00:00Z (out of order: below 2026-10-03T02:59:00Z before it)"), "{o}");
}

#[test]
fn a_column_that_is_not_sorted_has_no_order_to_break() {
    // amounts jump around: half the steps go down, so no row is "out of order"
    assert!(!view(&csv(300, None), Some("csv")).unwrap().contains("out of order"));
}
