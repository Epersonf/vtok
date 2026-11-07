use super::output::{select, Query};
use super::store::{latest_in, load_in, store_in, valid_id};

fn tmp(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("vtok-store-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

fn q() -> Query { Query { grep: vec![], range: None, tail: None, head: None, context: 1, max_tokens: 2000 } }

#[test]
fn stored_output_round_trips_and_latest_is_the_newest() {
    let d = tmp("latest");
    assert!(latest_in(&d).is_none(), "nothing stored yet");
    let a = store_in(&d, "first run\nline two\n").unwrap();
    let b = store_in(&d, "second run\n").unwrap();
    assert!(valid_id(&a) && valid_id(&b) && a != b);
    assert_eq!(load_in(&d, &a).unwrap(), vec!["first run".to_string(), "line two".to_string()]);
    assert_eq!(latest_in(&d).as_deref(), Some(b.as_str()));
    assert!(load_in(&d, "../x").is_err() && load_in(&d, "deadbeef").is_err());
    std::fs::remove_file(d.join("latest")).unwrap();
    assert!(latest_in(&d).is_some(), "without the pointer file the newest file wins");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn same_text_stored_twice_gets_two_ids() {
    let d = tmp("twice");
    let a = store_in(&d, "same\n").unwrap();
    let b = store_in(&d, "same\n").unwrap();
    assert_ne!(a, b);
    assert!(load_in(&d, &a).is_ok() && load_in(&d, &b).is_ok());
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn output_selects_grep_lines_tail_and_default() {
    let lines: Vec<String> = (1..=100).map(|i| if i == 50 { "boom here".to_string() } else { format!("line {i}") }).collect();
    let g = select("abc12345", &lines, &Query { grep: vec!["BOOM".into()], ..q() });
    assert!(g.contains("49: line 49") && g.contains("50: boom here") && g.contains("51: line 51") && !g.contains("52:"), "{g}");
    let t = select("abc12345", &lines, &Query { tail: Some(2), ..q() });
    assert!(t.contains("99: line 99") && t.contains("100: line 100") && !t.contains("98:"), "{t}");
    let r = select("abc12345", &lines, &Query { range: Some((3, 4)), ..q() });
    assert!(r.starts_with("# output abc12345: 100 lines\n") && r.contains("3: line 3") && r.contains("4: line 4"), "{r}");
    let d = select("abc12345", &lines, &q());
    assert!(d.contains("100: line 100") && d.contains("61: line 61") && !d.contains("60:"), "default is the last 40 lines: {d}");
    assert!(select("abc12345", &lines, &Query { grep: vec!["nothing".into()], ..q() }).contains("no matching lines"));
}
