use super::filter::Filter;
use super::gain::render;
use super::rec::Rec;
use super::store::Source;
use serde_json::Value;
use std::path::PathBuf;

fn rec(kind: &str, raw: u64, shown: u64, session: &str, tagged: bool) -> Rec {
    Rec { t: 1_000_000, kind: kind.into(), raw, shown, session: session.into(), cwd: "/p".into(), via: "cli".into(), tagged, ..Default::default() }
}

fn src(name: &str, recs: Vec<Rec>) -> Source { Source { path: PathBuf::from(name), recs } }

fn json(sources: &[Source], f: &Filter) -> Value { serde_json::from_str(&render(sources, f, 1_000_100, true)).unwrap() }

#[test]
fn only_exact_and_upper_bound_kinds_are_counted() {
    let s = [src("a", vec![rec("run", 1000, 200, "", true), rec("read", 500, 300, "", true), rec("find", 1_000_000, 50, "", true)])];
    let v = json(&s, &Filter::default());
    assert_eq!((v["calls"].as_u64(), v["raw"].as_u64(), v["shown"].as_u64()), (Some(2), Some(1500), Some(500)), "find is listed but not counted: {v}");
    assert_eq!((v["saved_exact"].as_i64(), v["saved_upper_bound"].as_i64()), (Some(800), Some(200)));
    let find = v["rows"].as_array().unwrap().iter().find(|r| r["kind"] == "find").unwrap();
    assert_eq!((find["class"].as_str(), find["saved"].as_i64()), (Some("not counted"), Some(0)));
}

#[test]
fn a_huge_output_counts_only_what_a_shell_tool_would_have_shown() {
    let s = [src("a", vec![rec("run", 14_800_000, 700, "", true)])];
    let v = json(&s, &Filter::default());
    assert_eq!(v["saved_exact"].as_i64(), Some(8_000 - 700), "{v}");
}

#[test]
fn older_untagged_records_are_left_out_and_counted_aside() {
    let s = [src("old", vec![rec("run", 275_000_000, 6_000_000, "", false)]), src("new", vec![rec("run", 900, 100, "chat", true)])];
    let v = json(&s, &Filter::default());
    assert_eq!((v["calls"].as_u64(), v["legacy_skipped"].as_u64()), (Some(1), Some(1)));
    let all = json(&s, &Filter { legacy: true, ..Default::default() });
    assert_eq!(all["calls"].as_u64(), Some(2));
    let text = render(&s, &Filter::default(), 1_000_100, false);
    assert!(text.contains("1 older records without a session tag were left out"), "{text}");
}

#[test]
fn a_chat_filter_keeps_only_its_calls() {
    let s = [src("a", vec![rec("run", 900, 100, "chat-a", true), rec("run", 900, 100, "chat-b", true), rec("read", 400, 100, "chat-a", true)])];
    let v = json(&s, &Filter { session: Some("chat-a".into()), ..Default::default() });
    assert_eq!(v["calls"].as_u64(), Some(2));
}

#[test]
fn the_text_says_what_each_raw_means_and_where_it_read() {
    let s = [src("C:/logs/usage.jsonl", vec![rec("run", 900, 100, "", true), rec("find", 5000, 40, "", true)])];
    let t = render(&s, &Filter::default(), 1_000_100, false);
    assert!(t.contains("C:/logs/usage.jsonl (2 records)"), "{t}");
    assert!(t.contains("exact: the command's own output") && t.contains("not counted: the alternative is a whole project"), "{t}");
    assert!(t.contains("saved 800 (exact 800, upper bound 0)") && t.contains("A/B"), "{t}");
}

#[test]
fn calls_printed_as_they_were_are_counted_apart() {
    let tiny = Rec { detail: "ls #ab12cd34 (tiny)".into(), ..rec("run", 100, 100, "", true) };
    let same = Rec { detail: "cargo test #cd34ab12 (not smaller)".into(), ..rec("run", 900, 900, "", true) };
    let v = json(&[src("a", vec![tiny, same, rec("run", 900, 100, "", true)])], &Filter::default());
    assert_eq!((v["passed_through"].as_u64(), v["calls"].as_u64(), v["saved_exact"].as_i64()), (Some(2), Some(3), Some(800)), "{v}");
}

#[test]
fn nothing_recorded_names_the_places_it_looked() {
    let t = render(&[src("empty.jsonl", vec![])], &Filter::default(), 1_000_100, false);
    assert!(t.starts_with("no usage recorded") && t.contains("empty.jsonl"), "{t}");
}
