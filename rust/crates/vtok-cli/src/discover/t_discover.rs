//! The scan counts each tool call once, asks the hook per call, and sorts Read results by kind of file.
use super::scan::scan;
use serde_json::{json, Value};
use std::path::PathBuf;

fn output(lines: usize) -> String {
    (0..lines).map(|i| format!("line {i}: some output that repeats a little and carries the number {}\n", i * 7)).collect()
}

/// The two transcript lines of one tool call: the request and its result.
fn call(id: &str, name: &str, input: Value, out: &str, cwd: &str) -> [String; 2] {
    let line = |kind: &str, block: Value| json!({"type": kind, "cwd": cwd, "message": {"content": [block]}}).to_string();
    [
        line("assistant", json!({"type": "tool_use", "id": id, "name": name, "input": input})),
        line("user", json!({"type": "tool_result", "tool_use_id": id, "content": out})),
    ]
}

fn dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("vtok-disc-{tag}-{}", std::process::id()));
    std::fs::remove_dir_all(&d).ok();
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn a_call_repeated_in_another_transcript_counts_once_and_the_hook_is_asked_per_call() {
    let d = dir("once");
    let cwd = d.to_str().unwrap();
    let mut lines = vec![];
    lines.extend(call("t1", "Bash", json!({"command": "cargo test"}), &output(300), cwd));
    lines.extend(call("t2", "Bash", json!({"command": "cat notes.txt"}), &output(300), cwd));
    lines.extend(call("t3", "Read", json!({"file_path": "docs/story.md"}), &output(300), cwd));
    lines.extend(call("t4", "Bash", json!({"command": "echo hi"}), "hi", cwd));
    let text = lines.join("\n");
    let (a, b) = (d.join("a.jsonl"), d.join("b.jsonl"));
    std::fs::write(&a, &text).unwrap();
    std::fs::write(&b, &text).unwrap(); // a resumed session repeats the same calls
    let s = scan(&[a, b], 400);
    assert_eq!((s.files, s.shell_calls, s.big.len()), (2, 3, 2));
    assert!(s.big.iter().any(|c| c.key == "cargo test" && c.hooked && c.shown < c.raw));
    assert!(s.big.iter().any(|c| c.key == "cat" && !c.hooked));
    assert_eq!((s.reads.calls, s.reads.prose > 0, s.reads.code + s.reads.data), (1, true, 0));
    std::fs::remove_dir_all(&d).ok();
}

#[test]
fn the_recorded_directory_decides_whether_the_hook_wraps_a_cat_of_a_big_data_file() {
    let d = dir("cwd");
    let table: String = (0..2500).map(|i| format!("{i},north,{},{}\n", i * 3, i % 7)).collect();
    std::fs::write(d.join("sales.csv"), format!("id,region,amount,code\n{table}")).unwrap();
    let cwd = d.to_str().unwrap();
    let mut lines = vec![];
    lines.extend(call("c1", "Bash", json!({"command": "cat sales.csv"}), &output(300), cwd));
    lines.extend(call("c2", "Bash", json!({"command": "cat sales.csv"}), &output(300), "/nowhere/at/all"));
    let f = d.join("t.jsonl");
    std::fs::write(&f, lines.join("\n")).unwrap();
    let s = scan(&[f], 400);
    assert_eq!(s.big.iter().map(|c| c.hooked).collect::<Vec<_>>(), vec![true, false]);
    std::fs::remove_dir_all(&d).ok();
}

#[test]
fn a_command_only_the_wide_rules_wrap_is_marked_apart() {
    let d = dir("wide");
    std::fs::write(d.join(".vtok.json"), r#"{"hook_wide": 0}"#).unwrap(); // the recorded directory decides, not this machine
    let cwd = d.to_str().unwrap();
    let mut lines = vec![];
    // the last two: the line is credited to its main command, the first one that is not a `cd` or `export`
    let cmds = ["cargo test", "grep -n line src/a.rs", "python make_report.py", "sed -n 1,50p notes.md && ls -la", "cd sub && export A=1 && ls -la"];
    for (i, cmd) in cmds.iter().enumerate() {
        lines.extend(call(&format!("w{i}"), "Bash", json!({"command": cmd}), &output(300), cwd));
    }
    let f = d.join("t.jsonl");
    std::fs::write(&f, lines.join("\n")).unwrap();
    let flags: Vec<(bool, bool)> = scan(&[f], 400).big.iter().map(|c| (c.hooked, c.wide)).collect();
    assert_eq!(flags, [(true, true), (false, true), (false, false), (false, false), (false, true)]);
    std::fs::remove_dir_all(&d).ok();
}

#[test]
fn a_leading_cd_does_not_name_the_command() {
    let d = dir("cd");
    let cwd = d.to_str().unwrap();
    let cmds = ["cd /x/y; cargo test", "cd \"/x y/z\" && cd sub && cargo test", "cd /x/y\ncargo test", "cd /a && for f in 1 2; do cat $f; done", "export PYTHONIOENCODING=utf-8; cargo test"];
    let mut lines = vec![];
    for (i, cmd) in cmds.iter().enumerate() {
        lines.extend(call(&format!("d{i}"), "Bash", json!({"command": cmd}), &output(300), cwd));
    }
    let f = d.join("t.jsonl");
    std::fs::write(&f, lines.join("\n")).unwrap();
    let keys: Vec<String> = scan(&[f], 400).big.iter().map(|c| c.key.clone()).collect();
    assert_eq!(keys, ["cargo test", "cargo test", "cargo test", "shell loop", "cargo test"]);
    std::fs::remove_dir_all(&d).ok();
}

#[test]
fn reads_are_sorted_by_kind_of_file() {
    let d = dir("reads");
    let cwd = d.to_str().unwrap();
    let mut lines = vec![];
    for (i, path) in ["a.rs", "b.json", "c.txt", "d.xyz"].iter().enumerate() {
        lines.extend(call(&format!("r{i}"), "Read", json!({"file_path": path}), &output(300), cwd));
    }
    let f = d.join("t.jsonl");
    std::fs::write(&f, lines.join("\n")).unwrap();
    let r = scan(&[f], 400).reads;
    assert_eq!(r.calls, 4);
    assert!(r.code > 0 && r.data > 0 && r.prose > 0 && r.other > 0, "one of each");
    std::fs::remove_dir_all(&d).ok();
}
