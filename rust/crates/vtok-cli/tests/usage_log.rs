//! The usage log, end to end with the real binary: a `vtok run` is recorded with the chat's session id, the directory and
//! the kind of command (never its arguments), `gain` reads it back by chat, and two processes that disagree about HOME
//! still share one state directory.
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const EXE: &str = env!("CARGO_BIN_EXE_vtok");

fn scratch(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("vtok-log-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// The vtok binary with a state directory of its own and a chat session id; nothing from the real environment leaks in.
fn vtok(state: &Path, session: &str, args: &[&str]) -> Output {
    Command::new(EXE).args(args).env("VTOK_STATE", state).env("VTOK_SESSION", session)
        .env_remove("VTOK_NO_USAGE").env_remove("CLAUDE_CODE_SESSION_ID").output().unwrap()
}

fn text(o: &Output) -> String { String::from_utf8_lossy(&o.stdout).into_owned() }

fn records(state: &Path) -> Vec<serde_json::Value> {
    std::fs::read_to_string(state.join("usage.jsonl")).unwrap_or_default().lines().filter_map(|l| serde_json::from_str(l).ok()).collect()
}

#[test]
fn a_run_is_recorded_with_chat_directory_and_command_kind() {
    let state = scratch("run");
    let o = vtok(&state, "chat-one", &["run", "--", EXE, "grammar", "--lang", "rust"]);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let recs = records(&state);
    assert_eq!(recs.len(), 1, "{recs:?}");
    let r = &recs[0];
    assert_eq!((r["v"].as_u64(), r["kind"].as_str(), r["s"].as_str(), r["via"].as_str()), (Some(2), Some("run"), Some("chat-one"), Some("cli")));
    assert!(r["cwd"].as_str().is_some_and(|c| !c.is_empty()));
    let detail = r["detail"].as_str().unwrap();
    assert!(detail.starts_with("vtok grammar #") && !detail.contains("rust"), "the kind of command, not its arguments: {detail}");
    let _ = std::fs::remove_dir_all(&state);
}

#[test]
fn gain_reads_one_chat_back_and_leaves_the_others_out() {
    let state = scratch("gain");
    vtok(&state, "chat-a", &["run", "--", EXE, "languages"]);
    vtok(&state, "chat-b", &["run", "--", EXE, "languages"]);
    let all = text(&vtok(&state, "chat-a", &["gain", "--json"]));
    let one = text(&vtok(&state, "chat-a", &["gain", "--json", "--session", "current"]));
    let calls = |j: &str| serde_json::from_str::<serde_json::Value>(j).unwrap()["rows"][0]["calls"].as_u64();
    assert_eq!((calls(&all), calls(&one)), (Some(2), Some(1)), "{all} / {one}");
    let _ = std::fs::remove_dir_all(&state);
}

/// The binary as a process the way a shell or a hook sees it (HOME set) or the way an MCP server a desktop app starts sees
/// it (no HOME), both under the same `%LOCALAPPDATA%`.
#[cfg(windows)]
fn on_windows(local: &Path, home: Option<&Path>, args: &[&str]) -> Output {
    let mut c = Command::new(EXE);
    c.args(args).env("LOCALAPPDATA", local).env("VTOK_SESSION", "x").env_remove("VTOK_STATE").env_remove("XDG_STATE_HOME").env_remove("VTOK_NO_USAGE");
    match home { Some(h) => { c.env("HOME", h).env("USERPROFILE", h); } None => { c.env_remove("HOME").env_remove("USERPROFILE"); } }
    c.output().unwrap()
}

#[cfg(windows)]
#[test]
fn processes_with_and_without_home_share_one_log_and_one_output_store() {
    let (local, home) = (scratch("localappdata"), scratch("fakehome"));
    on_windows(&local, Some(&home), &["run", "--", EXE, "grammar", "--lang", "rust"]);
    on_windows(&local, None, &["run", "--", EXE, "grammar", "--lang", "rust"]);
    let log = std::fs::read_to_string(local.join("vtok").join("usage.jsonl")).unwrap();
    assert_eq!(log.lines().count(), 2, "both processes wrote the same log: {log}");
    assert!(!home.join(".local/state/vtok/usage.jsonl").exists(), "nothing went to the HOME-based directory");
    let read_back = on_windows(&local, None, &["output", "--tail", "1"]);
    assert!(read_back.status.success() && text(&read_back).starts_with("# output "), "{}", String::from_utf8_lossy(&read_back.stderr));
    let _ = (std::fs::remove_dir_all(&local), std::fs::remove_dir_all(&home));
}

#[test]
fn no_usage_switch_keeps_the_log_empty() {
    let state = scratch("off");
    let o = Command::new(EXE).args(["run", "--", EXE, "languages"]).env("VTOK_STATE", &state).env("VTOK_NO_USAGE", "1").output().unwrap();
    assert!(o.status.success());
    assert!(records(&state).is_empty());
    let _ = std::fs::remove_dir_all(&state);
}
