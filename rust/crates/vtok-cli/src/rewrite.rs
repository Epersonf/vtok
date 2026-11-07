//! Command rewriting shared by `vtok hook` and `vtok rewrite`: decide which shell commands print a lot and wrap them in
//! `vtok run --`. Compound lines (`a && b`, `a; b`, `a || b`) are rewritten segment by segment; a segment with a pipe,
//! redirect, substitution or subshell is left alone, and so is any line with a background `&`, heredoc or newline.
//! A bare `cat` of a large CSV, JSON or YAML file is wrapped too (see `files`). With the `hook-wide` setting on, `wide` adds
//! grep, ls, wc, more git subcommands and big log files; `shell` reads the line itself.
use std::path::{Path, PathBuf};

mod files;
mod shell;
mod wide;
#[cfg(test)] mod t_files;
#[cfg(test)] mod t_rewrite;
#[cfg(test)] mod t_wide;
pub use shell::split_prefix;
use shell::{has_shell_meta, segments};

fn words(cmd: &str) -> Vec<&str> {
    // skip leading VAR=value assignments
    cmd.split_whitespace().skip_while(|w| w.contains('=') && !w.starts_with('-') && !w.contains('/') || (w.contains('=') && w.chars().next().map_or(false, |c| c.is_ascii_uppercase()))).collect()
}

/// True for commands whose output is usually long and mostly noise (the default rule set).
pub fn is_chatty(cmd: &str) -> bool { is_chatty_in(cmd, None, false) }

/// `base`: the directory the shell is in, when it is known (a `cat` of a relative file name depends on it). `wide`: the
/// `hook-wide` rules count too.
fn is_chatty_in(cmd: &str, base: Option<&Path>, wide: bool) -> bool {
    if crate::tomlfilters::matches(cmd.trim()) { return true; }
    let w = words(cmd);
    let Some(&prog) = w.first() else { return false };
    let prog = prog.rsplit('/').next().unwrap_or(prog);
    let sub = w.get(1).copied().unwrap_or("");
    let third = w.get(2).copied().unwrap_or("");
    let has = |x: &str| w.iter().skip(1).any(|a| *a == x);
    let narrow = match prog {
        "pytest" | "py.test" | "jest" | "vitest" | "mocha" | "tsc" | "eslint" | "ruff" | "mypy" | "pylint" | "flake8" | "make" | "mvn" | "gradle" | "gradlew" | "tox" | "nox" | "rspec" | "phpunit" | "playwright" | "cypress" | "golangci-lint" | "rubocop" | "prettier" | "biome" | "pyright" | "terraform" | "tofu" | "helm" | "bazel" | "ctest" | "cmake" | "msbuild" => true,
        "python" | "python3" => sub == "-m" && matches!(third, "pytest" | "unittest" | "pip" | "mypy" | "ruff" | "tox"),
        "npx" | "bunx" | "pnpx" => matches!(sub, "jest" | "vitest" | "tsc" | "eslint" | "playwright" | "prettier" | "cypress" | "mocha" | "biome"),
        "cargo" => matches!(sub, "test" | "build" | "check" | "clippy" | "doc" | "bench" | "install" | "update" | "fetch" | "nextest" | "audit" | "fmt" | "tree"),
        "npm" | "pnpm" | "yarn" | "bun" => matches!(sub, "test" | "install" | "i" | "ci" | "add" | "build" | "lint" | "audit" | "outdated" | "ls" | "list") || (sub == "run" && matches!(third, "test" | "build" | "lint" | "typecheck")),
        "pip" | "pip3" | "uv" | "poetry" => matches!(sub, "install" | "list" | "freeze" | "sync" | "lock" | "add") || (sub == "pip" && matches!(third, "install" | "list" | "freeze")),
        "go" => matches!(sub, "test" | "build" | "vet" | "mod" | "get"),
        "dotnet" => matches!(sub, "test" | "build" | "restore" | "publish"),
        "docker" | "podman" => matches!(sub, "build" | "logs" | "pull" | "ps" | "images" | "push") || (sub == "compose" && matches!(third, "build" | "logs" | "up" | "pull" | "ps")),
        "docker-compose" => matches!(sub, "build" | "logs" | "up" | "pull" | "ps"),
        "kubectl" => matches!(sub, "logs" | "get" | "describe"),
        "gh" => (matches!(sub, "pr" | "run" | "issue") && matches!(third, "list" | "view" | "checks" | "diff")) || (sub == "run" && third == "watch"),
        "aws" | "gcloud" | "az" => w.len() > 2,
        "git" => matches!(sub, "log" | "diff" | "show" | "status" | "blame") && !has("--oneline") && !has("-1") && !has("-n1") && !w.iter().any(|a| a.starts_with("--format") || a.starts_with("--pretty") || *a == "--quiet" || *a == "-q"),
        "find" | "tree" => true,
        "cat" => files::big_data_file(&w[1..], base),
        "ls" => w.iter().skip(1).any(|a| a.starts_with('-') && a.contains('R')),
        "grep" | "rg" => w.iter().skip(1).any(|a| a.starts_with('-') && (a.contains('r') || a.contains('R'))) || prog == "rg",
        _ => false,
    };
    narrow || (wide && wide::chatty(&w, base))
}

fn wrap(seg: &str, low_risk_only: bool, base: Option<&Path>, wide: bool) -> Option<String> {
    let t = seg.trim();
    if t.is_empty() || t.starts_with("vtok ") || has_shell_meta(t) || !is_chatty_in(t, base, wide) || (low_risk_only && !crate::risk::low_risk(t)) { return None; }
    Some(seg.replacen(t, &format!("vtok run -- {t}"), 1))
}

/// The rewritten command line, or None to leave the original alone.
pub fn rewrite(cmd: &str) -> Option<String> { rewrite_with(cmd, false) }

/// `low_risk_only`: wrap only read-only / build-and-test segments (for hooks whose protocol implies "allow").
pub fn rewrite_with(cmd: &str, low_risk_only: bool) -> Option<String> { rewrite_in(cmd, low_risk_only, None) }

/// Like `rewrite_with`, for a shell that is in `cwd` (the hook input says where); without it, this process's directory.
/// A `cd DIR` segment moves the directory that later segments read their files from.
pub fn rewrite_in(cmd: &str, low_risk_only: bool, cwd: Option<&str>) -> Option<String> {
    let wide = wide::mentions(cmd) && crate::config::hook_wide(cwd.map(Path::new));
    rewrite_opt(cmd, low_risk_only, cwd, wide)
}

/// Like `rewrite_in`, with the `hook-wide` choice made by the caller (`vtok discover` asks both ways; the tests pin it).
pub fn rewrite_opt(cmd: &str, low_risk_only: bool, cwd: Option<&str>, wide: bool) -> Option<String> {
    let segs = segments(cmd)?;
    let mut base: Option<PathBuf> = cwd.map(PathBuf::from).or_else(|| std::env::current_dir().ok());
    let mut changed = false;
    let mut out = String::new();
    for (s, sep) in &segs {
        base = files::after_cd(base, s);
        match wrap(s, low_risk_only, base.as_deref(), wide) { Some(w) => { changed = true; out.push_str(&w); } None => out.push_str(s) }
        out.push_str(sep);
    }
    changed.then_some(out)
}

/// Whether the hook would wrap the main command of a line: its first segment that is not a `cd DIR` or `export X=y`.
/// `vtok discover` uses it, because a recorded output does not say which segment printed what: crediting the whole output
/// to a trailing `ls` would make a `sed` that ran first look wrapped.
pub fn wraps_main(cmd: &str, cwd: Option<&str>, wide: bool) -> bool {
    let Some(segs) = segments(cmd) else { return false };
    let mut base: Option<PathBuf> = cwd.map(PathBuf::from).or_else(|| std::env::current_dir().ok());
    for (s, _) in &segs {
        base = files::after_cd(base, s);
        if matches!(s.split_whitespace().next(), Some("cd" | "export")) { continue; }
        return wrap(s, false, base.as_deref(), wide).is_some();
    }
    false
}

/// `vtok rewrite CMD...`: prints the rewritten line and exits 0, or exits 1 with no output when nothing should change.
/// Agent-neutral entry point for hook systems other than Claude Code's.
pub fn run(args: &[String]) -> i32 {
    match rewrite(&args.join(" ")) { Some(r) => { println!("{r}"); 0 } None => 1 }
}
