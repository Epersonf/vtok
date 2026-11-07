//! How the hook names this program: `vtok` when that resolves on PATH, the executable's own path when it does not.
use super::*;
use std::ffi::OsString;
use std::path::PathBuf;

/// A directory holding a file that counts as `vtok` on this platform, and a PATH value made of just that directory.
fn path_with_vtok(tag: &str) -> (PathBuf, OsString) {
    let d = std::env::temp_dir().join(format!("vtok-bin-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join(if cfg!(windows) { "vtok.exe" } else { "vtok" }), b"").unwrap();
    let path = std::env::join_paths([&d]).unwrap();
    (d, path)
}

fn empty_path() -> OsString { std::env::join_paths([std::env::temp_dir().join("vtok-no-such-dir")]).unwrap() }

#[test]
fn vtok_on_path_keeps_the_short_name() {
    let (d, path) = path_with_vtok("on");
    assert_eq!(binary_for(Some(&path), Some(PathBuf::from("C:/x/vtok.exe"))), "vtok");
    std::fs::remove_dir_all(&d).ok();
}

#[test]
fn off_path_the_executable_is_named_by_its_path() {
    let none = empty_path();
    assert_eq!(binary_for(Some(&none), Some(PathBuf::from(r"C:\Users\me\.vtok\bin\vtok.exe"))), "C:/Users/me/.vtok/bin/vtok.exe");
    assert_eq!(binary_for(None, Some(PathBuf::from("/opt/vtok/vtok"))), "/opt/vtok/vtok");
    assert_eq!(binary_for(Some(&none), Some(PathBuf::from(r"C:\Program Files\vtok\vtok.exe"))), "\"C:/Program Files/vtok/vtok.exe\"");
    assert_eq!(binary_for(Some(&none), None), "vtok");
}

#[test]
fn has_hook_recognises_vtok_by_name_or_path_and_nothing_else() {
    let entry = |c: &str| json!({"matcher": "Bash", "hooks": [{"type": "command", "command": c}]});
    let cmd = "C:/Users/me/.vtok/bin/vtok.exe hook";
    assert!(has_hook(&entry("vtok hook"), cmd));
    assert!(has_hook(&entry(cmd), "other"));
    assert!(has_hook(&entry("\"C:/Program Files/vtok/vtok.exe\" hook"), "other"));
    assert!(has_hook(&entry("C:\\tools\\vtok.exe hook"), "other"));
    assert!(has_hook(&entry("renamed hook"), "renamed hook"), "the very command we would write counts");
    assert!(!has_hook(&entry("my-vtok-wrapper hook"), cmd));
    assert!(!has_hook(&entry("vtok run"), cmd));
    assert!(!has_hook(&json!({"matcher": "Edit", "hooks": []}), cmd));
    assert!(!has_hook(&json!({"matcher": "Edit"}), cmd));
}

#[test]
fn install_writes_the_command_once_even_when_another_vtok_path_is_registered() {
    let p = std::env::temp_dir().join(format!("vtok-hook-bin-{}.json", std::process::id()));
    std::fs::write(&p, r#"{"theme":"dark"}"#).unwrap();
    let ps = p.to_str().unwrap();
    assert!(install_with(ps, "C:/a/vtok.exe hook").unwrap());
    assert!(!install_with(ps, "C:/a/vtok.exe hook").unwrap());
    assert!(!install_with(ps, "vtok hook").unwrap(), "a vtok hook is already registered");
    let v: Value = serde_json::from_str(&std::fs::read_to_string(&p).unwrap()).unwrap();
    assert_eq!(v["theme"], "dark");
    assert_eq!(v["hooks"]["PreToolUse"].as_array().unwrap().len(), 1);
    assert_eq!(v["hooks"]["PreToolUse"][0]["hooks"][0]["command"], "C:/a/vtok.exe hook");
    std::fs::remove_file(&p).ok();
}
