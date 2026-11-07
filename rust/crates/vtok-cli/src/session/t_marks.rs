use super::marks::*;
use std::io::Write;

fn scope(chat: &str) -> Scope { Scope { chat: chat.into(), cwd: "/p".into() } }

fn mark(t: u64, start: bool, name: &str, s: &Scope) -> Mark { Mark { t, start, name: name.into(), scope: s.clone() } }

#[test]
fn a_start_and_a_stop_make_one_closed_stretch() {
    let a = scope("chat-a");
    let got = spans(&[mark(100, true, "fix login", &a), mark(160, false, "fix login", &a)]);
    assert_eq!(got, vec![Span { name: "fix login".into(), from: 100, to: Some(160), scope: a }]);
}

#[test]
fn a_new_start_closes_the_open_stretch_of_the_same_chat_only() {
    let (a, b) = (scope("chat-a"), scope("chat-b"));
    let s = spans(&[mark(10, true, "one", &a), mark(20, true, "other chat", &b), mark(30, true, "two", &a)]);
    assert_eq!((s[0].to, s[1].to, s[2].to), (Some(30), None, None));
    assert_eq!(open_span(&s, &a).map(|x| x.name.as_str()), Some("two"));
    assert_eq!(open_span(&s, &b).map(|x| x.name.as_str()), Some("other chat"));
}

#[test]
fn without_a_chat_the_directory_is_the_scope() {
    let (x, y) = (Scope { chat: String::new(), cwd: "/p".into() }, Scope { chat: String::new(), cwd: "/q".into() });
    assert!(x.same(&x.clone()) && !x.same(&y));
    assert!(!x.same(&scope("chat-a")), "a chat never matches a directory-only scope, even in the same directory");
}

#[test]
fn a_stretch_is_picked_by_name_or_as_the_newest() {
    let a = scope("chat-a");
    let s = spans(&[mark(1, true, "alpha", &a), mark(5, false, "alpha", &a), mark(9, true, "beta", &a), mark(12, false, "beta", &a)]);
    assert_eq!(pick(&s, &a, None).map(|x| x.name.as_str()), Some("beta"));
    assert_eq!(pick(&s, &a, Some("alpha")).map(|x| x.from), Some(1));
    assert!(pick(&s, &a, Some("gamma")).is_none() && pick(&s, &scope("chat-b"), None).is_none());
}

#[test]
fn a_stop_with_nothing_open_changes_nothing() {
    assert!(spans(&[mark(5, false, "x", &scope("c"))]).is_empty());
}

#[test]
fn names_are_trimmed_cut_and_never_empty() {
    assert_eq!(clean_name("  fix\tthe\nlogin  ").unwrap(), "fix the login");
    assert!(clean_name("   ").is_err() && clean_name("").is_err());
    assert_eq!(clean_name(&"x".repeat(100)).unwrap().chars().count(), 60);
}

#[test]
fn marks_survive_the_file_and_a_damaged_line_is_skipped() {
    let dir = std::env::temp_dir().join(format!("vtok-marks-{}", std::process::id()));
    let path = dir.join("marks.jsonl");
    let a = scope("chat-a");
    append(&path, &mark(7, true, "task", &a)).unwrap();
    std::fs::OpenOptions::new().append(true).open(&path).unwrap().write_all(b"not json\n").unwrap();
    append(&path, &mark(9, false, "task", &a)).unwrap();
    assert_eq!(read(&path), vec![mark(7, true, "task", &a), mark(9, false, "task", &a)]);
    let _ = std::fs::remove_dir_all(&dir);
}
