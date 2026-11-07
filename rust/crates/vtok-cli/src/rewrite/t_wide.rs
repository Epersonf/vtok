//! The `hook-wide` rule set: what it adds, what it still leaves alone, and that the setting decides when the caller does not.
use super::*;
use std::path::PathBuf;

fn wide(cmd: &str) -> Option<String> { rewrite_opt(cmd, false, None, true) }
fn narrow(cmd: &str) -> Option<String> { rewrite_opt(cmd, false, None, false) }

/// A scratch directory with 36,000-byte files of several kinds and a small log.
fn scratch(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("vtok-wide-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    for name in ["big.log", "big.diff", "BIG.LOG", "big.md", "big.rs", "big.txt"] { std::fs::write(d.join(name), "a line\n".repeat(5200)).unwrap(); }
    std::fs::write(d.join("small.log"), "a line\n".repeat(100)).unwrap();
    d
}

#[test]
fn grep_ls_and_wc_are_wrapped_when_the_option_is_on_and_not_otherwise() {
    for cmd in ["grep -n foo src/a.rs", "grep foo a b", "egrep -i foo f", "ls", "ls -la src", "wc -l src/a.rs src/b.rs", "FOO=1 grep x f", "cd app && ls"] {
        assert!(wide(cmd).is_some_and(|r| r.contains("vtok run -- ")), "{cmd}");
        assert!(narrow(cmd).is_none(), "{cmd}");
    }
    assert_eq!(wide("grep -n foo src/a.rs").as_deref(), Some("vtok run -- grep -n foo src/a.rs"));
    for no in ["ls | head", "wc -l < f", "grep foo f | sort", "ls > out.txt", "vtok run -- ls"] { assert!(wide(no).is_none(), "{no}"); }
}

#[test]
fn a_grep_that_only_counts_or_lists_names_stays_as_typed() {
    for no in ["grep -q foo f", "grep -c foo f", "grep -l foo a b", "grep -L foo a b", "grep --count foo f", "grep -nc foo f", "grep --quiet x f", "git grep -c foo", "git grep -q foo"] {
        assert!(wide(no).is_none(), "{no}");
    }
}

#[test]
fn more_git_subcommands_that_only_read() {
    for yes in ["git ls-files", "git ls-tree -r HEAD", "git shortlog -sn", "git rev-list HEAD", "git grep -n foo", "git reflog", "git stash list", "git stash show -p", "git log --oneline"] {
        assert!(wide(yes).is_some(), "{yes}");
        assert!(narrow(yes).is_none(), "{yes}");
    }
    let no = ["git branch", "git branch -D x", "git tag v1", "git stash", "git stash pop", "git reflog expire --all", "git rev-list --count HEAD", "git config --list"];
    for no in no { assert!(wide(no).is_none(), "{no}"); }
}

#[test]
fn a_bounded_or_formatted_log_is_left_alone() {
    for no in ["git log --oneline -20", "git log --oneline -n 5", "git log --oneline -n5", "git log --oneline --max-count=3", "git log --oneline --format=%H", "git log --oneline --pretty=short", "git log --oneline -q"] {
        assert!(wide(no).is_none(), "{no}");
    }
    assert!(wide("git log --stat").is_some() && narrow("git log --stat").is_some());
}

#[test]
fn cat_of_a_big_log_but_not_of_prose_or_code() {
    let d = scratch("cat");
    let at = |cmd: &str| rewrite_opt(cmd, false, d.to_str(), true);
    for yes in ["cat big.log", "cat big.diff", "cat BIG.LOG", "cat 'big.log'"] { assert_eq!(at(yes), Some(format!("vtok run -- {yes}")), "{yes}"); }
    for no in ["cat big.md", "cat big.rs", "cat big.txt", "cat small.log", "cat missing.log", "cat -n big.log", "cat big.log big.diff", "cat big.log | head"] { assert!(at(no).is_none(), "{no}"); }
    assert!(rewrite_opt("cat big.log", false, d.to_str(), false).is_none());
    std::fs::remove_dir_all(&d).ok();
}

#[test]
fn agents_whose_hook_implies_allow_get_only_what_is_read_only() {
    for yes in ["ls -la", "grep -n foo f", "git log --oneline"] { assert!(rewrite_opt(yes, true, None, true).is_some(), "{yes}"); }
    for no in ["wc -l f", "git ls-files", "git stash list", "git grep foo"] { assert!(rewrite_opt(no, true, None, true).is_none(), "{no}"); }
}

#[test]
fn the_setting_decides_when_the_caller_does_not() {
    let d = scratch("setting");
    let at = |json: &str| {
        std::fs::write(d.join(".vtok.json"), json).unwrap();
        rewrite_in("ls -la", false, d.to_str())
    };
    assert_eq!(at(r#"{"hook_wide": 1}"#).as_deref(), Some("vtok run -- ls -la"));
    assert_eq!(at(r#"{"hook_wide": 0}"#), None);
    assert_eq!(at(r#"{"hook_wide": 7}"#), None); // out of range: ignored, the default is off
    assert!(rewrite_in("cargo test", false, d.to_str()).is_some(), "the default rules do not depend on it");
    std::fs::remove_dir_all(&d).ok();
}

#[test]
fn only_lines_with_a_known_program_read_the_setting() {
    for yes in ["ls", "cd x && grep a b", "FOO=1 wc -l f", "echo hi; git ls-files", "/usr/bin/cat f", "(ls)"] { assert!(wide::mentions(yes), "{yes}"); }
    for no in ["cargo test", "echo hi", "python x.py", "pytest -v"] { assert!(!wide::mentions(no), "{no}"); }
}
