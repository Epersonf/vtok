use super::transcript::*;
use crate::usage::fmt::utc;

fn assistant(id: &str, ts: &str, output: u64, sidechain: bool) -> String {
    format!(r#"{{"type":"assistant","isSidechain":{sidechain},"timestamp":"{ts}","message":{{"id":"{id}","model":"claude-x","usage":{{"input_tokens":3,"cache_read_input_tokens":500,"cache_creation_input_tokens":100,"output_tokens":{output}}}}}}}"#)
}

#[test]
fn timestamps_and_utc_text_are_inverse() {
    let t = iso_secs("2026-11-07T03:48:45.199Z").unwrap();
    assert!((t - 1_794_023_325.199).abs() < 1e-3, "{t}");
    assert_eq!(utc(t as u64), "2026-11-07 03:48");
    assert_eq!(iso_secs("1970-01-01T00:00:00.000Z"), Some(0.0));
    assert_eq!(utc(951_782_400), "2000-02-29 00:00", "a leap day");
    assert!(iso_secs("yesterday").is_none());
}

#[test]
fn one_message_written_as_several_lines_counts_once_with_its_largest_usage() {
    let text = [assistant("m1", "2026-11-07T03:00:05.000Z", 5, false), assistant("m1", "2026-11-07T03:00:03.000Z", 40, false), assistant("m2", "2026-11-07T03:01:00.000Z", 7, false)].join("\n");
    let tr = parse(&text);
    assert_eq!(tr.turns.len(), 2);
    assert_eq!((tr.turns[0].usage.output, tr.turns[1].usage.output), (40, 7));
    assert_eq!(tr.turns[0].t, iso_secs("2026-11-07T03:00:03.000Z").unwrap(), "the earliest line gives the time");
    assert_eq!((tr.turns[0].usage.cache_read, tr.turns[0].usage.cache_write, tr.turns[0].usage.input), (500, 100, 3));
    assert_eq!(tr.model.as_deref(), Some("claude-x"));
}

#[test]
fn subagent_lines_and_lines_that_are_not_turns_are_left_out() {
    let text = [assistant("s1", "2026-11-07T03:00:00.000Z", 9, true), "not json at all".to_string(),
                r#"{"type":"system","timestamp":"2026-11-07T03:00:01.000Z"}"#.to_string(), assistant("m1", "2026-11-07T03:00:02.000Z", 1, false)].join("\n");
    assert_eq!(parse(&text).turns.len(), 1);
}

#[test]
fn prompts_are_what_a_human_typed_and_compactions_are_marked() {
    let typed = r#"{"type":"user","timestamp":"2026-11-07T03:00:10.000Z","origin":{"kind":"human"},"message":{"role":"user","content":"fix the login"}}"#;
    let blocks = r#"{"type":"user","timestamp":"2026-11-07T03:00:20.000Z","origin":{"kind":"human"},"message":{"role":"user","content":[{"type":"text","text":"and the logout"}]}}"#;
    let print_mode = r#"{"type":"user","timestamp":"2026-11-07T03:00:25.000Z","turnOrigin":"sdk","message":{"role":"user","content":"claude -p prompt"}}"#;
    let peer = r#"{"type":"user","timestamp":"2026-11-07T03:00:26.000Z","turnOrigin":"peer","message":{"role":"user","content":"from another session"}}"#;
    let meta = r#"{"type":"user","isMeta":true,"timestamp":"2026-11-07T03:00:27.000Z","origin":{"kind":"human"},"message":{"role":"user","content":"injected"}}"#;
    let tool = r#"{"type":"user","timestamp":"2026-11-07T03:00:30.000Z","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"x","content":"ok"}]}}"#;
    let summary = r#"{"type":"user","isCompactSummary":true,"timestamp":"2026-11-07T03:00:40.000Z","message":{"role":"user","content":"summary"}}"#;
    let tr = parse(&[typed, blocks, print_mode, peer, meta, tool, summary].join("\n"));
    let texts: Vec<&str> = tr.prompts.iter().map(|p| p.text.as_str()).collect();
    assert_eq!(texts, ["fix the login", "and the logout", "claude -p prompt"], "not a tool result, injected context or another session's message");
    assert_eq!(tr.compactions, vec![iso_secs("2026-11-07T03:00:40.000Z").unwrap()]);
}

#[test]
fn a_transcript_is_looked_up_only_for_a_plain_session_id() {
    assert!(find("").is_none() && find("../secrets").is_none() && find("a/b").is_none() && find("a\\b").is_none());
}
