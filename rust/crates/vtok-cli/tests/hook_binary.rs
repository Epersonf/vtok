//! The hook names the program by its path when `vtok` is not on PATH, and by `vtok` when it is. Runs the real binary with
//! a PATH that is either an empty directory or a directory holding a stand-in file named `vtok`.
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

const EXE: &str = env!("CARGO_BIN_EXE_vtok");
const INPUT: &str = r#"{"tool_name":"Bash","tool_input":{"command":"cargo test --all"}}"#;

fn scratch(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("vtok-it-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// This executable the way the hook writes it: `/` separators, quoted when the path has a space.
fn own_name() -> String {
    let s = EXE.replace('\\', "/");
    if s.contains(' ') { format!("\"{s}\"") } else { s }
}

fn run(args: &[&str], stdin: &str, path_dir: &Path) -> Output {
    let mut child = Command::new(EXE).args(args).env("PATH", path_dir).env_remove("VTOK_HOOK_WIDE")
        .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
    child.stdin.take().unwrap().write_all(stdin.as_bytes()).unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    out
}

fn rewritten(out: &Output) -> String {
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    v["hookSpecificOutput"]["updatedInput"]["command"].as_str().unwrap().to_string()
}

#[test]
fn off_path_the_rewritten_command_runs_this_executable_by_path() {
    let empty = scratch("empty");
    assert_eq!(rewritten(&run(&["hook"], INPUT, &empty)), format!("{} run -- cargo test --all", own_name()));
    std::fs::remove_dir_all(&empty).ok();
}

#[test]
fn on_path_the_rewritten_command_says_vtok() {
    let dir = scratch("onpath");
    std::fs::write(dir.join(if cfg!(windows) { "vtok.exe" } else { "vtok" }), b"").unwrap();
    assert_eq!(rewritten(&run(&["hook"], INPUT, &dir)), "vtok run -- cargo test --all");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_command_already_wrapped_by_path_is_left_alone() {
    let empty = scratch("again");
    let input = format!(r#"{{"tool_name":"Bash","tool_input":{{"command":"{} run -- cargo test"}}}}"#, EXE.replace('\\', "/"));
    assert!(run(&["hook"], &input, &empty).stdout.is_empty());
    std::fs::remove_dir_all(&empty).ok();
}

#[test]
fn init_registers_the_hook_by_path_once() {
    let dir = scratch("init");
    let settings = dir.join("settings.json");
    std::fs::write(&settings, r#"{"theme":"dark"}"#).unwrap();
    let s = settings.to_str().unwrap();
    assert!(String::from_utf8_lossy(&run(&["init", "--claude-hook", s], "", &dir).stdout).contains("hook added"));
    assert!(String::from_utf8_lossy(&run(&["init", "--claude-hook", s], "", &dir).stdout).contains("already up to date"));
    let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&settings).unwrap()).unwrap();
    assert_eq!(v["theme"], "dark");
    assert_eq!(v["hooks"]["PreToolUse"][0]["hooks"][0]["command"], format!("{} hook", own_name()));
    std::fs::remove_dir_all(&dir).ok();
}
