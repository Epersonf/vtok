//! `vtok statusline` and `vtok init --claude-statusline`, end to end with the real binary.
use serde_json::Value;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

const EXE: &str = env!("CARGO_BIN_EXE_vtok");

fn scratch(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("vtok-line-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn base(state: &Path) -> Command {
    let mut c = Command::new(EXE);
    c.env("VTOK_STATE", state).env("CLAUDE_CONFIG_DIR", state).env_remove("VTOK_NO_USAGE").env_remove("CLAUDE_CODE_SESSION_ID").env_remove("VTOK_SESSION");
    c
}

/// `vtok statusline` with what Claude Code sends on stdin.
fn statusline(state: &Path, stdin: &str) -> String {
    let mut child = base(state).arg("statusline").stdin(Stdio::piped()).stdout(Stdio::piped()).spawn().unwrap();
    child.stdin.take().unwrap().write_all(stdin.as_bytes()).unwrap();
    String::from_utf8_lossy(&child.wait_with_output().unwrap().stdout).into_owned()
}

fn log(state: &Path, lines: &[&str]) { std::fs::write(state.join("usage.jsonl"), lines.join("\n") + "\n").unwrap(); }

fn call(t: u64, kind: &str, raw: u64, shown: u64, chat: &str) -> String {
    format!(r#"{{"v":2,"t":{t},"kind":"{kind}","raw":{raw},"shown":{shown},"detail":"x","s":"{chat}","cwd":"/p","via":"cli"}}"#)
}

#[test]
fn the_line_follows_the_chat_the_agent_names() {
    let state = scratch("chat");
    log(&state, &[&call(1_700_000_000, "run", 6000, 900, "chat-one"), &call(1_700_000_010, "read", 800, 300, "chat-one"), &call(1_700_000_020, "run", 4000, 1000, "chat-two")]);
    assert_eq!(statusline(&state, r#"{"session_id":"chat-one","cwd":"/p"}"#).trim(), "vtok saved 5.1k-5.6k tok · 2 calls");
    assert_eq!(statusline(&state, r#"{"session_id":"chat-two"}"#).trim(), "vtok saved 3.0k tok · 1 calls");
    assert_eq!(statusline(&state, r#"{"session_id":"nobody"}"#), "", "nothing to say, nothing printed");
    assert_eq!(statusline(&state, "not json at all"), "", "no chat and no calls in this directory");
    let _ = std::fs::remove_dir_all(&state);
}

#[test]
fn an_open_stretch_narrows_the_line_to_itself() {
    let state = scratch("open");
    log(&state, &[&call(1000, "run", 9000, 1000, "chat-one"), &call(2000, "run", 5000, 1000, "chat-one")]);
    std::fs::write(state.join("marks.jsonl"), r#"{"t":1500,"op":"start","name":"work","s":"chat-one","cwd":"/p"}"#.to_string() + "\n").unwrap();
    assert_eq!(statusline(&state, r#"{"session_id":"chat-one"}"#).trim(), "vtok[work] saved 4.0k tok · 1 calls");
    let _ = std::fs::remove_dir_all(&state);
}

fn init(state: &Path, settings: &Path) -> Output { base(state).args(["init", "--claude-statusline", settings.to_str().unwrap()]).output().unwrap() }

#[test]
fn init_adds_the_status_line_only_when_none_is_set() {
    let dir = scratch("init");
    let settings = dir.join("settings.json");
    let first = init(&dir, &settings);
    let said = String::from_utf8_lossy(&first.stdout).into_owned();
    assert!(first.status.success() && said.contains("status line added as"), "{said}");
    let v: Value = serde_json::from_str(&std::fs::read_to_string(&settings).unwrap()).unwrap();
    assert!(v["statusLine"]["command"].as_str().unwrap().ends_with(" statusline"), "{v}");
    assert!(String::from_utf8_lossy(&init(&dir, &settings).stdout).contains("already up to date"));
    std::fs::write(&settings, r#"{"statusLine":{"type":"command","command":"my-line.sh"}}"#).unwrap();
    assert!(String::from_utf8_lossy(&init(&dir, &settings).stdout).contains("a status line is already set (`my-line.sh`)"));
    assert!(std::fs::read_to_string(&settings).unwrap().contains("my-line.sh"));
    let _ = std::fs::remove_dir_all(&dir);
}
