//! Through `vtok run`: the data view is chosen by the file a command reads or by the shape of the output.
use super::t_csv::csv;
use super::t_json::deployments;
use super::t_yaml::services;
use crate::shell::preview;

#[test]
fn cat_of_a_csv_keeps_the_odd_row_and_the_header() {
    let o = preview("cat data.csv", &csv(1800, Some(1137)).join("\n"), 0, 700);
    assert!(o.starts_with("csv: 1800 rows, 7 fields\nL1: txn_id,"), "{o}");
    assert!(o.contains("L1139: T901137,") && o.contains("# more: vtok output"), "{o}");
}

#[test]
fn a_forty_row_csv_is_no_longer_reduced_to_its_last_row() {
    let o = preview("cat small.csv", &csv(40, None).join("\n"), 0, 700);
    assert!(o.contains("L1: txn_id,") && o.contains("L2: T900000,"), "{o}");
}

#[test]
fn csv_from_any_command_is_recognised_by_its_shape() {
    let o = preview("export-report --all", &csv(1800, Some(900)).join("\n"), 0, 700);
    assert!(o.contains("L902: T900900,") && o.contains("country=XX"), "{o}");
}

#[test]
fn json_and_yaml_files() {
    let doc = serde_json::to_string_pretty(&deployments(130, 88)).unwrap();
    let o = preview("cat deploy.json", &doc, 0, 700);
    assert!(o.contains("items[88] name=deploy-088: resources.limits.cpu=4000m"), "{o}");
    let o = preview("type compose.yaml", &services(45, 31).join("\n"), 0, 700);
    assert!(o.contains("svc-31: image=registry.local/svc-31:1.9.0-rc2"), "{o}");
}

#[test]
fn a_data_file_that_is_no_table_keeps_its_first_and_last_lines() {
    // quoted fields with line breaks: most lines have the wrong number of fields, so there is no table to describe
    let mut rows = vec!["id,note,when".to_string()];
    for i in 0..120 { rows.extend([format!("{i},\"first line of {i}"), "second line".to_string(), "third, still the note\",2026-01-01".to_string()]); }
    let o = preview("cat notes.csv", &rows.join("\n"), 0, 700);
    assert!(o.starts_with("id,note,when\n") && o.contains("lines omitted") && o.contains("third, still the note"), "{o}");
    assert!(super::cut(&rows, Some("rs")).is_none() && super::cut(&rows[..40], Some("csv")).is_none());
}

#[test]
fn only_files_of_unknown_layout_are_judged_by_their_text() {
    let table = csv(300, Some(150)).join("\n");
    // the same text is a table when the file is a plain-text dump or the command names no file at all
    for cmd in ["cat dump.txt", "cat export.dat", "export-report"] { assert!(preview(cmd, &table, 0, 700).starts_with("csv: 300 rows"), "{cmd}"); }
    // but a source or document file is never read as a table, whatever its lines look like
    for cmd in ["cat notes.md", "cat main.rs", "cat build.gradle", "cat page.html"] { assert!(!preview(cmd, &table, 0, 700).starts_with("csv: "), "{cmd}"); }
}

#[test]
fn source_files_and_plain_json_keep_their_old_views() {
    let rs: String = (0..400).map(|i| format!("let v{i} = {i};\n")).collect();
    assert!(preview("cat big.ts", &rs, 0, 700).contains("lines omitted"));
    let cfg = serde_json::to_string_pretty(&serde_json::json!({"name": "x", "deps": {"a": "1"}, "files": ["a"], "n": 1, "m": 2, "k": 3, "j": 4, "i": 5, "h": 6, "g": 7})).unwrap();
    assert!(!preview("cat package.json", &cfg, 0, 700).contains("records"));
}
