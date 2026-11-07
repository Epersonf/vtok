use super::by_prompt::{json_of, lines, table};
use super::chat::{measure, Standing};
use super::render::text;
use super::report::{Opts, Report};
use super::transcript::{Prompt, Transcript, Turn, Usage};
use crate::usage::rec::Rec;
use crate::usage::tally::tally;
use std::path::PathBuf;

fn turn(t: f64) -> Turn { Turn { t, usage: Usage { input: 0, cache_read: 1000, cache_write: 0, output: 10 } } }

/// Two prompts, at 100 and 300 s: two turns after the first, three after the second, each turn costing 1,500 units.
fn transcript() -> Transcript {
    let prompt = |t: f64, text: &str| Prompt { t, text: text.into() };
    Transcript { turns: [110.0, 150.0, 310.0, 350.0, 390.0].map(turn).to_vec(), prompts: vec![prompt(100.0, "fix the login"), prompt(300.0, "and the logout")], ..Default::default() }
}

fn rec(t: u64, kind: &str, raw: u64, shown: u64) -> Rec {
    Rec { t, kind: kind.into(), raw, shown, session: "abcdef123456".into(), cwd: "/p".into(), via: "mcp".into(), tagged: true, ..Default::default() }
}

fn two_calls() -> Vec<Rec> { vec![rec(120, "run", 3000, 1000), rec(320, "read", 800, 300)] }

fn mcp(seen: bool) -> Standing { Standing { seen, tokens: 729.0 } }

fn all_lines() -> Vec<super::by_prompt::Line> { lines(&transcript(), &two_calls().iter().collect::<Vec<_>>(), 100.0, 400.0, mcp(true)) }

#[test]
fn each_prompt_gets_its_own_turns_and_calls_but_a_saving_counts_to_the_end() {
    let l = all_lines();
    assert_eq!(l.len(), 2);
    assert_eq!((l[0].chat.turns, l[0].calls, l[0].tokens, l[0].chat.units), (2, 1, 2000, 3000.0));
    assert_eq!((l[1].chat.turns, l[1].calls, l[1].tokens, l[1].chat.units), (3, 1, 500, 4500.0));
    assert_eq!(l[0].chat.gain_exact, 2000.0 * 15.5, "the run in the first prompt rides the four turns after it, two of them in the second prompt");
    assert_eq!(l[1].chat.gain_upper, 500.0 * 13.5);
    assert_eq!((l[0].chat.standing, l[1].chat.standing), (729.0 * 13.5, 729.0 * 3.0), "only the first prompt has the first turn, which wrote the instructions");
}

#[test]
fn a_prompt_with_nothing_in_the_window_is_left_out() {
    let l = lines(&transcript(), &[], 200.0, 400.0, mcp(false));
    assert_eq!(l.len(), 1, "the first prompt's turns are before the window");
    assert_eq!(l[0].text, "and the logout");
}

#[test]
fn the_table_puts_the_dearest_prompt_first_and_honours_the_limit() {
    let t = table(all_lines(), 100.0, &Opts::default());
    assert!(t.contains("by prompt, dearest first (2 of 2)") && t.contains("+03:20") && t.contains("US$0.001"), "{t}");
    assert!(t.find("and the logout") < t.find("fix the login"), "{t}");
    let one = table(all_lines(), 100.0, &Opts { limit: Some(1), ..Opts::default() });
    assert!(one.contains("(1 of 2)") && !one.contains("fix the login"), "{one}");
}

#[test]
fn the_report_prints_and_exports_the_prompts_when_asked() {
    let recs = two_calls();
    let chat = measure(&transcript(), &recs.iter().collect::<Vec<_>>(), 100.0, 400.0, mcp(true));
    let rows = tally(recs.iter());
    let r = Report { title: "whole chat".into(), chat_id: "abcdef123456".into(), open: false, from: 100, to: 400, recs, rows, chat: Some(chat), transcript: Some((PathBuf::from("x.jsonl"), transcript())) };
    let o = Opts { by_prompt: true, ..Opts::default() };
    assert!(text(&r, &o).contains("by prompt, dearest first") && !text(&r, &Opts::default()).contains("by prompt"));
    let j = json_of(&all_lines(), 100.0, &o);
    assert_eq!((j[0]["prompt"].as_str(), j[0]["at"].as_f64(), j[1]["turns"].as_u64()), (Some("fix the login"), Some(0.0), Some(3)));
}
