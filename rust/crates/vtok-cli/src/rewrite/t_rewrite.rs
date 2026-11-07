//! Which command lines are wrapped in `vtok run --`, and which are left exactly as typed.
use super::*;

/// The default rule set, whatever the `hook-wide` setting of the machine that runs the tests.
fn rewrite(cmd: &str) -> Option<String> { rewrite_opt(cmd, false, None, false) }

#[test]
fn wraps_chatty_commands_only() {
    assert_eq!(rewrite("cargo test --release").as_deref(), Some("vtok run -- cargo test --release"));
    assert_eq!(rewrite("FOO=1 python -m pytest -v tests").as_deref(), Some("vtok run -- FOO=1 python -m pytest -v tests"));
    assert!(rewrite("npm install").is_some() && rewrite("git log --stat").is_some() && rewrite("grep -rn foo src").is_some());
    for no in ["ls", "cat a.txt", "git log --oneline", "git status -q", "cargo test | head", "vtok run -- cargo test", "python script.py", "cd x\ncargo test", "grep foo file", "cargo run", "cargo test &", "cargo test > out.txt"] {
        assert!(rewrite(no).is_none(), "{no}");
    }
}

#[test]
fn compound_lines_are_rewritten_per_segment() {
    assert_eq!(rewrite("cd app && cargo build && cargo test").as_deref(), Some("cd app && vtok run -- cargo build && vtok run -- cargo test"));
    assert_eq!(rewrite("npm ci; npm test").as_deref(), Some("vtok run -- npm ci; vtok run -- npm test"));
    assert_eq!(rewrite("cargo build && cargo test | tail").as_deref(), Some("vtok run -- cargo build && cargo test | tail"));
    assert_eq!(rewrite("grep -rn \"a|b\" src").as_deref(), Some("vtok run -- grep -rn \"a|b\" src"));
    assert_eq!(rewrite("echo 'a && cargo test'"), None);
    assert_eq!(rewrite("cargo test \"unbalanced"), None);
    assert_eq!(rewrite("cargo test && echo $(date)"), Some("vtok run -- cargo test && echo $(date)".into()));
}

#[test]
fn newer_tools_are_known() {
    for yes in ["gh pr checks 12", "terraform plan", "docker compose logs web", "npx vitest run", "cargo nextest run", "aws s3 ls"] { assert!(rewrite(yes).is_some(), "{yes}"); }
    for no in ["gh auth status", "docker compose down", "npx create-app", "aws"] { assert!(rewrite(no).is_none(), "{no}"); }
}
