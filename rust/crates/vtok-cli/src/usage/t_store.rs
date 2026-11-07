use super::kinds::{class, counted, saved, Class, READ_CAP, RUN_CAP};
use super::rec::Rec;
use super::store::{append, read_file};

fn tmp(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("vtok-usage-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

fn rec(kind: &str, raw: u64, shown: u64) -> Rec {
    Rec { t: 42, kind: kind.into(), raw, shown, detail: "d".into(), session: "chat-1".into(), cwd: "/p".into(), via: "mcp".into(), tagged: true }
}

#[test]
fn a_record_survives_the_trip_through_a_file_with_its_tags() {
    let d = tmp("trip");
    let p = d.join("usage.jsonl");
    append(&p, &rec("read", 100, 40)).unwrap();
    append(&p, &Rec { session: String::new(), ..rec("run", 9, 9) }).unwrap();
    let back = read_file(&p);
    assert_eq!(back.len(), 2);
    assert_eq!(back[0], rec("read", 100, 40));
    assert_eq!(back[1].session, "", "an unknown chat stays empty");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn older_lines_without_tags_are_read_as_untagged_and_junk_is_skipped() {
    let d = tmp("legacy");
    std::fs::create_dir_all(&d).unwrap();
    let p = d.join("usage.jsonl");
    std::fs::write(&p, "{\"t\":5,\"kind\":\"run\",\"raw\":10,\"shown\":2,\"detail\":\"x\"}\nnot json\n{\"kind\":\"run\"}\n").unwrap();
    let back = read_file(&p);
    assert_eq!(back.len(), 1, "a line without a time is not a record");
    assert!(!back[0].tagged && back[0].session.is_empty());
    assert!(read_file(&d.join("missing.jsonl")).is_empty());
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn details_are_cut_to_eighty_characters() {
    let r = Rec::now("run", 1, 1, &"x".repeat(200));
    assert_eq!(r.detail.chars().count(), 80);
    assert!(r.tagged && !r.cwd.is_empty());
}

#[test]
fn kinds_fall_in_three_classes_and_caps_follow_the_tool() {
    assert_eq!(class("run"), Class::Exact);
    assert_eq!((class("read"), class("chunk"), class("outline")), (Class::Upper, Class::Upper, Class::Upper));
    for k in ["find", "refs", "map", "graph", "browse", "route", "output", "edit"] { assert_eq!(class(k), Class::NotCounted, "{k}"); }
    assert_eq!(counted("run", 50_000), RUN_CAP);
    assert_eq!(counted("read", 50_000), READ_CAP);
    assert_eq!(counted("find", 50_000), 50_000);
    assert_eq!(saved("run", 50_000, 700), RUN_CAP as i64 - 700);
    assert_eq!(saved("find", 50_000, 10), 0);
    assert_eq!(saved("read", 100, 130), -30, "a compact view that came out larger is a loss");
}
