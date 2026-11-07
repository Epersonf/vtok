//! The wider rule set of the hook, behind the `hook-wide` setting (off by default): grep, ls and wc, more read-only git
//! subcommands, and a plain `cat` of a big log. `vtok run` prints an output of up to `run-tiny` tokens unchanged, so a short
//! result costs nothing; what these commands share is that a long one is mostly lines nobody reads one by one. Source code and
//! prose are not here on purpose: `vtok run` shows the ends of a long output, and whoever cats a file to read it needs the
//! middle too.
use super::files;
use std::path::Path;

fn program(word: &str) -> &str {
    word.rsplit('/').next().unwrap_or(word)
}

/// Whether `cmd` names a program these rules know: a cheap test before the setting is read.
pub fn mentions(cmd: &str) -> bool {
    cmd.split(|c: char| c.is_whitespace() || ";&|()".contains(c)).any(|t| matches!(program(t), "grep" | "egrep" | "fgrep" | "ls" | "wc" | "git" | "cat"))
}

/// Whether the words of one command (leading `VAR=value` skipped) are wrapped by the wide rules.
pub fn chatty(w: &[&str], base: Option<&Path>) -> bool {
    let Some(&prog) = w.first() else { return false };
    let args = &w[1..];
    match program(prog) {
        "ls" | "wc" => true,
        "grep" | "egrep" | "fgrep" => !args.iter().any(|a| counts_only(a)),
        "git" => git(args),
        "cat" => files::sole_file(args, base).is_some_and(|(name, len)| len >= files::BIG && log_like(&name)),
        _ => false,
    }
}

/// A grep flag that makes the output a count, a list of file names or nothing: short already.
fn counts_only(a: &str) -> bool {
    match a.strip_prefix("--") {
        Some(long) => matches!(long, "quiet" | "silent" | "count" | "files-with-matches" | "files-without-match"),
        None => a.strip_prefix('-').is_some_and(|short| short.chars().any(|c| matches!(c, 'q' | 'c' | 'l' | 'L'))),
    }
}

/// The git subcommands the default rules do not know and that only read, plus an unbounded `log --oneline`.
fn git(args: &[&str]) -> bool {
    let has = |x: &str| args.iter().any(|a| *a == x);
    match args.first().copied().unwrap_or("") {
        "ls-files" | "ls-tree" | "shortlog" | "rev-list" => !has("-q") && !has("--quiet") && !has("--count"),
        "grep" => !args[1..].iter().any(|a| counts_only(a)),
        "reflog" => !has("expire") && !has("delete"),
        "stash" => matches!(args.get(1).copied(), Some("list" | "show")),
        "log" => has("--oneline") && !limited(args) && !args.iter().any(|a| a.starts_with("--format") || a.starts_with("--pretty") || *a == "--quiet" || *a == "-q"),
        _ => false,
    }
}

/// A `git log` count limit: `-20`, `-n 20`, `-n20`, `--max-count=20`.
fn limited(args: &[&str]) -> bool {
    args.iter().any(|a| a.starts_with("--max-count") || a.strip_prefix('-').is_some_and(|r| r.starts_with(|c: char| c.is_ascii_digit() || c == 'n')))
}

/// Files made of events, not of text someone wrote: logs, traces, dumps, diffs.
fn log_like(name: &str) -> bool {
    name.rsplit_once('.').is_some_and(|(_, ext)| matches!(ext.to_ascii_lowercase().as_str(), "log" | "out" | "err" | "trace" | "dump" | "diff" | "patch"))
}
