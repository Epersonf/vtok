use super::chat::{measure, Chat};
use super::json::json;
use super::render::text;
use super::report::{Opts, Report};
use super::transcript::{Transcript, Turn, Usage};
use crate::usage::rec::Rec;
use crate::usage::tally::tally;
use std::path::PathBuf;

fn turn(t: f64) -> Turn { Turn { t, usage: Usage { input: 0, cache_read: 1000, cache_write: 0, output: 10 } } }

/// Five turns, 100 s apart, each costing 1,500 units (1,000 cache reads and 10 output tokens).
fn transcript(compactions: &[f64]) -> Transcript {
    Transcript { turns: (1..=5).map(|i| turn(i as f64 * 100.0)).collect(), compactions: compactions.to_vec(), ..Default::default() }
}

fn rec(t: u64, kind: &str, raw: u64, shown: u64) -> Rec {
    Rec { t, kind: kind.into(), raw, shown, session: "abcdef123456".into(), cwd: "/p".into(), via: "mcp".into(), tagged: true, ..Default::default() }
}

fn measured(recs: &[Rec], tr: &Transcript, from: f64, to: f64, mcp: bool) -> Chat { measure(tr, &recs.iter().collect::<Vec<_>>(), from, to, mcp) }

fn two_calls() -> Vec<Rec> { vec![rec(150, "run", 3000, 1000), rec(250, "read", 800, 300)] }

#[test]
fn a_saved_token_is_worth_one_write_and_a_read_for_every_later_turn() {
    let c = measured(&two_calls(), &transcript(&[]), 100.0, 500.0, true);
    assert_eq!((c.turns, c.units), (5, 7500.0));
    assert_eq!(c.gain_exact, 2000.0 * 15.5, "the run at 150 rides 4 later turns: one write and three reads");
    assert_eq!(c.gain_upper, 500.0 * 14.5, "the read at 250 rides 3");
    assert_eq!(c.standing, 729.0 * (5.0 + 11.5), "five reads of the instructions, and the first turn also wrote them");
    assert_eq!((c.net_exact(), c.net_all()), (31_000.0 - 12_028.5, 31_000.0 + 7_250.0 - 12_028.5));
}

#[test]
fn a_compaction_throws_the_old_tool_results_away() {
    let c = measured(&two_calls(), &transcript(&[350.0]), 100.0, 500.0, true);
    assert_eq!(c.gain_exact, 2000.0 * 13.5, "the run rides the turns at 200 and 300 only: one write and one read");
    assert_eq!(c.gain_upper, 500.0 * 12.5, "the read rides the turn at 300 only");
}

#[test]
fn the_window_ends_where_the_report_ends() {
    let c = measured(&two_calls(), &transcript(&[]), 100.0, 300.0, true);
    assert_eq!(c.turns, 3);
    assert_eq!(c.gain_exact, 2000.0 * 13.5, "turns after the window are not counted, neither their cost nor their saving");
}

#[test]
fn the_write_of_the_instructions_is_charged_only_to_a_window_with_the_first_turn() {
    let tr = transcript(&[]);
    assert_eq!(measured(&[], &tr, 200.0, 500.0, true).standing, 729.0 * 4.0, "four turns, none of them the first");
    let none = measured(&[], &tr, 200.0, 500.0, false);
    assert_eq!((none.standing, none.mcp), (0.0, false), "no MCP in the chat, nothing to charge");
}

#[test]
fn only_counted_kinds_in_a_turn_that_follows_are_worth_anything() {
    let recs = [rec(150, "find", 1_000_000, 50), rec(500, "run", 3000, 1000)];
    let c = measured(&recs, &transcript(&[]), 100.0, 500.0, false);
    assert_eq!((c.gain_exact, c.gain_upper), (0.0, 0.0), "find has no alternative to compare with, and a run after the last turn has not paid yet");
}

fn report(recs: Vec<Rec>, chat: Option<Chat>) -> Report {
    let rows = tally(recs.iter());
    let transcript = chat.as_ref().map(|_| (PathBuf::from("abcdef123456.jsonl"), Transcript::default()));
    Report { title: "session \"login\"".into(), chat_id: "abcdef123456".into(), open: false, from: 100, to: 700, recs, rows, chat, transcript }
}

fn full_report() -> Report {
    let recs = two_calls();
    let chat = measured(&recs, &transcript(&[]), 100.0, 500.0, true);
    report(recs, Some(chat))
}

#[test]
fn the_text_sets_savings_against_cost_and_says_what_it_cannot_see() {
    let t = text(&full_report(), &Opts::default());
    assert!(t.starts_with("vtok session \"login\" · chat abcdef12 · 10 min · 2 calls"), "{t}");
    assert!(t.contains("exact calls (vtok run)") && t.contains("+31,000") && t.contains("+7,250") && t.contains("-12,029"), "{t}");
    assert!(t.contains("net: +18,972 units") && t.contains("+26,222"), "{t}");
    assert!(t.contains("only an A/B run"), "{t}");
}

#[test]
fn calls_that_did_not_pay_are_named_and_flagged_one_by_one() {
    let r = report(vec![rec(150, "read", 100, 150), rec(160, "run", 900, 100)], None);
    let t = text(&r, &Opts { per_call: true, ..Opts::default() });
    assert!(t.contains("did not pay (1 of 2 counted calls): read"), "{t}");
    assert!(t.contains("LARGER than the alternative") && t.contains("+00:50"), "{t}");
}

#[test]
fn without_calls_or_a_transcript_the_text_says_why() {
    assert!(text(&report(vec![], None), &Opts::default()).contains("no vtok calls recorded here yet"));
    assert!(text(&report(two_calls(), None), &Opts::default()).contains("No transcript found for chat abcdef12"));
}

#[test]
fn the_json_carries_the_same_numbers() {
    let v: serde_json::Value = serde_json::from_str(&json(&full_report(), &Opts::default())).unwrap();
    assert_eq!((v["calls"].as_u64(), v["saved_exact"].as_i64(), v["saved_upper_bound"].as_i64()), (Some(2), Some(2000), Some(500)), "{v}");
    assert_eq!((v["cost"]["turns"].as_u64(), v["cost"]["units"].as_f64(), v["cost"]["net_exact_units"].as_f64()), (Some(5), Some(7500.0), Some(18_971.5)), "{v}");
    assert_eq!(v["chat"], "abcdef123456");
}
