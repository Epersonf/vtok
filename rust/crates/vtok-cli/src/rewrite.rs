//! Command rewriting shared by `vtok hook` and `vtok rewrite`: decide which shell commands print a lot and wrap them in
//! `vtok run --`. Compound lines (`a && b`, `a; b`, `a || b`) are rewritten segment by segment; a segment with a pipe,
//! redirect, substitution or subshell is left alone, and so is any line with a background `&`, heredoc or newline.

fn words(cmd: &str) -> Vec<&str> {
    // skip leading VAR=value assignments
    cmd.split_whitespace().skip_while(|w| w.contains('=') && !w.starts_with('-') && !w.contains('/') || (w.contains('=') && w.chars().next().map_or(false, |c| c.is_ascii_uppercase()))).collect()
}

/// True for commands whose output is usually long and mostly noise.
pub fn is_chatty(cmd: &str) -> bool {
    let w = words(cmd);
    let Some(&prog) = w.first() else { return false };
    let prog = prog.rsplit('/').next().unwrap_or(prog);
    let sub = w.get(1).copied().unwrap_or("");
    let third = w.get(2).copied().unwrap_or("");
    let has = |x: &str| w.iter().skip(1).any(|a| *a == x);
    match prog {
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
        "ls" => w.iter().skip(1).any(|a| a.starts_with('-') && a.contains('R')),
        "grep" | "rg" => w.iter().skip(1).any(|a| a.starts_with('-') && (a.contains('r') || a.contains('R'))) || prog == "rg",
        _ => false,
    }
}

/// Split a line at top-level `&&`, `||` and `;`. Returns (segments with their trailing separator, ok). `ok` is false when the
/// line has something that makes segment-wise rewriting unsafe (background `&`, heredoc, newline, unbalanced quote).
fn segments(cmd: &str) -> Option<Vec<(String, &'static str)>> {
    let b: Vec<char> = cmd.chars().collect();
    let (mut out, mut cur, mut i) = (vec![], String::new(), 0);
    let mut q: Option<char> = None;
    while i < b.len() {
        let c = b[i];
        if let Some(qc) = q {
            cur.push(c);
            if c == '\\' && qc == '"' && i + 1 < b.len() { cur.push(b[i + 1]); i += 1; } else if c == qc { q = None; }
        } else if c == '\'' || c == '"' { q = Some(c); cur.push(c); }
        else if c == '\\' && i + 1 < b.len() { cur.push(c); cur.push(b[i + 1]); i += 1; }
        else if c == '\n' { return None; }
        else if c == '&' && b.get(i + 1) == Some(&'&') { out.push((std::mem::take(&mut cur), "&&")); i += 1; }
        else if c == '|' && b.get(i + 1) == Some(&'|') { out.push((std::mem::take(&mut cur), "||")); i += 1; }
        else if c == ';' { out.push((std::mem::take(&mut cur), ";")); }
        else if c == '&' || (c == '<' && b.get(i + 1) == Some(&'<')) { return None; }
        else { cur.push(c); }
        i += 1;
    }
    if q.is_some() { return None; }
    out.push((cur, ""));
    Some(out)
}

/// True when the segment has an unquoted pipe, redirect, backtick, substitution or grouping.
fn has_shell_meta(seg: &str) -> bool {
    let mut q: Option<char> = None;
    let mut esc = false;
    let b: Vec<char> = seg.chars().collect();
    for (i, &c) in b.iter().enumerate() {
        if esc { esc = false; continue; }
        match q {
            Some('\'') => { if c == '\'' { q = None; } }
            Some(_) => { if c == '\\' { esc = true; } else if c == '"' { q = None; } else if c == '`' || (c == '$' && b.get(i + 1) == Some(&'(')) { return true; } }
            None => match c {
                '\\' => esc = true,
                '\'' | '"' => q = Some(c),
                '|' | '<' | '>' | '`' | '(' | ')' | '{' | '}' => return true,
                '$' if b.get(i + 1) == Some(&'(') => return true,
                _ => {}
            },
        }
    }
    false
}

/// Split a leading `cd DIR && ` / `export X=y && ` prefix (agents add it to nearly every command) from the command itself.
pub fn split_prefix(cmd: &str) -> (&str, &str) {
    let mut rest = cmd.trim_start();
    let mut used = 0;
    while let Some(i) = rest.find("&&") {
        let head = rest[..i].trim();
        let simple = |h: &str| !h.is_empty() && !h.contains(['|', ';', '<', '>', '`', '\n', '(', ')', '{', '}', '&']) && !h.contains("$(");
        if (head.starts_with("cd ") || head.starts_with("export ")) && simple(head) {
            used += cmd.trim_start().len() - rest.len() + i + 2;
            rest = &rest[i + 2..];
        } else { break; }
    }
    let off = cmd.len() - cmd.trim_start().len() + used;
    (&cmd[..off.min(cmd.len())], cmd[off.min(cmd.len())..].trim_start())
}

fn wrap(seg: &str) -> Option<String> {
    let t = seg.trim();
    if t.is_empty() || t.starts_with("vtok ") || has_shell_meta(t) || !is_chatty(t) { return None; }
    Some(seg.replacen(t, &format!("vtok run -- {t}"), 1))
}

/// The rewritten command line, or None to leave the original alone.
pub fn rewrite(cmd: &str) -> Option<String> {
    let segs = segments(cmd)?;
    let mut changed = false;
    let mut out = String::new();
    for (s, sep) in &segs {
        match wrap(s) { Some(w) => { changed = true; out.push_str(&w); } None => out.push_str(s) }
        out.push_str(sep);
    }
    changed.then_some(out)
}

/// `vtok rewrite CMD...`: prints the rewritten line and exits 0, or exits 1 with no output when nothing should change.
/// Agent-neutral entry point for hook systems other than Claude Code's.
pub fn run(args: &[String]) -> i32 {
    match rewrite(&args.join(" ")) { Some(r) => { println!("{r}"); 0 } None => 1 }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
