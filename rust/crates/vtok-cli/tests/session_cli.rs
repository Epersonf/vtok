//! `vtok session`, end to end with the real binary: the report of a named stretch from a log and a transcript written by
//! hand (so the numbers are known), and the start / status / stop flow with the real clock.
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const EXE: &str = env!("CARGO_BIN_EXE_vtok");
/// 2023-11-14T22:13:20Z.
const BASE: u64 = 1_700_000_000;

fn scratch(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("vtok-sess-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// The binary with a state directory and a Claude config directory of its own; `chat` None means an agent with no session id.
fn vtok(state: &Path, claude: &Path, chat: Option<&str>, cwd: Option<&Path>, args: &[&str]) -> Output {
    let mut c = Command::new(EXE);
    c.args(args).env("VTOK_STATE", state).env("CLAUDE_CONFIG_DIR", claude).env_remove("VTOK_NO_USAGE").env_remove("CLAUDE_CODE_SESSION_ID").env_remove("VTOK_SESSION");
    if let Some(chat) = chat { c.env("VTOK_SESSION", chat); }
    if let Some(cwd) = cwd { c.current_dir(cwd); }
    c.output().unwrap()
}

fn text(o: &Output) -> String { String::from_utf8_lossy(&o.stdout).into_owned() }

fn iso(t: u64) -> String { let s = t - BASE + 13 * 60 + 20; format!("2023-11-14T22:{:02}:{:02}.000Z", s / 60, s % 60) }

/// Four calls (two inside the stretch "login", from +100 to +400 s) and six turns of 1,500 units each, 4 of them inside it.
fn fixture(state: &Path, claude: &Path) {
    let calls = [(50, "run", 5000, 500), (150, "run", 3000, 1000), (250, "read", 800, 300), (500, "run", 5000, 500)];
    let log: Vec<String> = calls.iter().map(|(t, k, raw, shown)| format!(r#"{{"v":2,"t":{},"kind":"{k}","raw":{raw},"shown":{shown},"detail":"x","s":"chat-one","cwd":"/p","via":"mcp"}}"#, BASE + t)).collect();
    std::fs::write(state.join("usage.jsonl"), log.join("\n") + "\n").unwrap();
    let mark = |t: u64, op: &str| format!(r#"{{"t":{},"op":"{op}","name":"login","s":"chat-one","cwd":"/p"}}"#, BASE + t);
    std::fs::write(state.join("marks.jsonl"), [mark(100, "start"), mark(400, "stop")].join("\n") + "\n").unwrap();
    let turns: Vec<String> = [60u64, 120, 200, 300, 380, 450].iter().enumerate().map(|(i, s)| format!(
        r#"{{"type":"assistant","timestamp":"{}","message":{{"id":"m{i}","model":"claude-x","usage":{{"input_tokens":0,"cache_read_input_tokens":1000,"cache_creation_input_tokens":0,"output_tokens":10}}}}}}"#, iso(BASE + s))).collect();
    let prompt = |t: u64, text: &str| format!(r#"{{"type":"user","timestamp":"{}","origin":{{"kind":"human"}},"message":{{"role":"user","content":"{text}"}}}}"#, iso(BASE + t));
    let lines = [turns, vec![prompt(55, "fix the login"), prompt(340, "and the logout")]].concat();
    let dir = claude.join("projects").join("-p");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("chat-one.jsonl"), lines.join("\n") + "\n").unwrap();
}

#[test]
fn a_named_stretch_is_reported_against_the_turns_of_its_transcript() {
    let (state, claude) = (scratch("fix"), scratch("fix-claude"));
    fixture(&state, &claude);
    let v: Value = serde_json::from_str(&text(&vtok(&state, &claude, Some("chat-one"), None, &["session", "report", "login", "--json"]))).unwrap();
    assert_eq!((v["calls"].as_u64(), v["saved_exact"].as_i64(), v["saved_upper_bound"].as_i64()), (Some(2), Some(2000), Some(500)), "{v}");
    let c = &v["cost"];
    assert_eq!((c["turns"].as_u64(), c["units"].as_f64()), (Some(4), Some(6000.0)), "{v}");
    assert_eq!((c["gain_exact_units"].as_f64(), c["gain_upper_units"].as_f64(), c["standing_units"].as_f64()), (Some(29_000.0), Some(6_750.0), Some(2_916.0)), "{v}");
    assert_eq!((c["net_exact_units"].as_f64(), c["net_all_units"].as_f64()), (Some(26_084.0), Some(32_834.0)), "{v}");
    let whole: Value = serde_json::from_str(&text(&vtok(&state, &claude, Some("chat-one"), None, &["session", "report", "--whole", "--json"]))).unwrap();
    assert_eq!((whole["calls"].as_u64(), whole["cost"]["turns"].as_u64()), (Some(4), Some(6)), "{whole}");
    let by: Value = serde_json::from_str(&text(&vtok(&state, &claude, Some("chat-one"), None, &["session", "report", "login", "--by-prompt", "--json"]))).unwrap();
    let p = by["prompts"].as_array().unwrap();
    assert_eq!((p.len(), p[0]["prompt"].as_str(), p[0]["turns"].as_u64(), p[0]["calls"].as_u64(), p[1]["turns"].as_u64()), (2, Some("fix the login"), Some(3), Some(2), Some(1)), "{by}");
    let t = text(&vtok(&state, &claude, Some("chat-one"), None, &["session", "report", "login", "--by-prompt", "--per-call"]));
    assert!(t.contains("by prompt, dearest first (2 of 2)") && t.contains("and the logout"), "{t}");
    assert!(t.contains("session \"login\"") && t.contains("2 calls") && t.contains("chat transcript chat-one.jsonl") && t.contains("net:"), "{t}");
    let _ = (std::fs::remove_dir_all(&state), std::fs::remove_dir_all(&claude));
}

#[test]
fn another_chat_does_not_see_a_stretch_it_did_not_start() {
    let (state, claude) = (scratch("other"), scratch("other-claude"));
    fixture(&state, &claude);
    let o = vtok(&state, &claude, Some("chat-two"), None, &["session", "report", "login"]);
    assert!(!o.status.success() && String::from_utf8_lossy(&o.stderr).contains("no session named"), "{}", String::from_utf8_lossy(&o.stderr));
    assert!(text(&vtok(&state, &claude, Some("chat-two"), None, &["session", "list"])).contains("no named sessions"));
    assert!(text(&vtok(&state, &claude, Some("chat-two"), None, &["session", "list", "--all"])).contains("login"));
    let _ = (std::fs::remove_dir_all(&state), std::fs::remove_dir_all(&claude));
}

#[test]
fn start_status_list_and_stop_follow_one_stretch() {
    let (state, claude) = (scratch("flow"), scratch("flow-claude"));
    let run = |args: &[&str]| vtok(&state, &claude, Some("chat-flow"), None, args);
    assert!(!run(&["session", "stop"]).status.success(), "nothing is open yet");
    let started = run(&["session", "start", "fix", "login"]);
    assert!(started.status.success() && text(&started).contains("\"fix login\" started"), "{}", text(&started));
    assert!(text(&run(&["session", "status"])).contains("session \"fix login\" open"));
    run(&["run", "--", EXE, "languages"]);
    let stopped = text(&run(&["session", "stop"]));
    assert!(stopped.contains("session \"fix login\"") && stopped.contains("1 calls") && stopped.contains("No transcript found"), "{stopped}");
    let list = text(&run(&["session", "list"]));
    assert!(list.contains("fix login") && !list.contains(" open"), "{list}");
    assert!(!run(&["session", "stop"]).status.success(), "it is already stopped");
    let _ = (std::fs::remove_dir_all(&state), std::fs::remove_dir_all(&claude));
}

#[test]
fn without_a_chat_id_the_calls_of_the_directory_count() {
    let (state, claude, here, there) = (scratch("dir"), scratch("dir-claude"), scratch("dir-a"), scratch("dir-b"));
    let run = |cwd: &Path, args: &[&str]| vtok(&state, &claude, None, Some(cwd), args);
    run(&here, &["session", "start", "dirwork"]);
    run(&here, &["run", "--", EXE, "languages"]);
    run(&there, &["run", "--", EXE, "languages"]);
    let t = text(&run(&here, &["session", "report"]));
    assert!(t.contains("session \"dirwork\" (open)") && t.contains("1 calls") && t.contains("No chat session id here"), "{t}");
    let _ = (std::fs::remove_dir_all(&state), std::fs::remove_dir_all(&claude), std::fs::remove_dir_all(&here), std::fs::remove_dir_all(&there));
}
