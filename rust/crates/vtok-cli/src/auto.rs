//! Agent-agnostic compaction: `vtok shims install` writes small wrapper scripts (cargo, npm, pytest, tsc, ...) into a
//! directory you put first on PATH. Any agent that runs shell commands then gets `vtok run` output with no hook and no
//! agent-specific setup. `vtok auto -- TOOL ARGS` is what a shim calls: it compacts only when the tool is a long-output
//! one AND stdout is not a terminal (a human at a prompt sees the real output); `VTOK_OFF=1` disables it.
//! Only test/build/lint/install tools are shimmed. Data producers (git, grep, find, ls) are not, because their output
//! is often piped into other programs and must stay exact; the hook's `vtok rewrite` handles those with pipe awareness.
use std::io::IsTerminal;
use std::path::{Path, PathBuf};

pub const TOOLS: &[&str] = &["pytest", "jest", "vitest", "mocha", "tsc", "eslint", "ruff", "mypy", "pylint", "flake8", "make", "mvn", "gradle", "tox", "nox", "rspec", "phpunit", "cargo", "npm", "pnpm", "yarn", "bun", "pip", "pip3", "uv", "poetry", "go", "dotnet", "docker", "kubectl", "terraform"];

fn is_shim(p: &Path) -> bool {
    std::fs::metadata(p).map_or(false, |m| m.is_file() && m.len() < 600) && std::fs::read_to_string(p).map_or(false, |t| t.contains("vtok auto"))
}

/// PATH without any directory that holds a vtok shim for `name`.
pub fn path_without_shims(name: &str, path: &std::ffi::OsStr) -> std::ffi::OsString {
    let keep: Vec<PathBuf> = std::env::split_paths(path).filter(|d| !is_shim(&d.join(name)) && !is_shim(&d.join(format!("{name}.cmd")))).collect();
    std::env::join_paths(keep).unwrap_or_else(|_| path.to_os_string())
}

pub fn run(args: &[String]) -> i32 {
    let args: &[String] = if args.first().map(|s| s.as_str()) == Some("--") { &args[1..] } else { args };
    let Some(name) = args.first() else { eprintln!("usage: vtok auto -- TOOL [ARGS...]"); return 2 };
    let rest = &args[1..];
    if let Some(p) = std::env::var_os("PATH") { std::env::set_var("PATH", path_without_shims(name, &p)); }
    let line = args.join(" ");
    let off = std::env::var_os("VTOK_OFF").is_some() || std::env::var_os("VTOK_WRAPPED").is_some();
    let human = std::io::stdout().is_terminal() && std::env::var_os("VTOK_FORCE").is_none();
    if !off && !human && crate::rewrite::is_chatty(&line) {
        std::env::set_var("VTOK_WRAPPED", "1"); // tools that call other shimmed tools are not compacted twice
        return crate::shell::run(&std::iter::once("--".to_string()).chain(args.iter().cloned()).collect::<Vec<_>>());
    }
    // through sh, like `vtok run`, so extensionless scripts resolve the same way on Windows
    match std::process::Command::new("sh").arg("-c").arg("exec \"$@\"").arg("sh").arg(name).args(rest).status() {
        Ok(s) => s.code().unwrap_or(1),
        Err(e) => { eprintln!("vtok auto: cannot run {name}: {e}"); 127 }
    }
}

fn default_dir() -> PathBuf {
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
    home.join(".vtok").join("shims")
}

/// Write the shim scripts into `dir`. Returns how many files were written.
pub fn install(dir: &Path) -> Result<usize, String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let mut n = 0;
    for t in TOOLS {
        let sh = dir.join(t);
        std::fs::write(&sh, "#!/bin/sh\nexec vtok auto -- \"$(basename \"$0\")\" \"$@\"\n").map_err(|e| format!("{}: {e}", sh.display()))?;
        #[cfg(unix)]
        { use std::os::unix::fs::PermissionsExt; let _ = std::fs::set_permissions(&sh, std::fs::Permissions::from_mode(0o755)); }
        std::fs::write(dir.join(format!("{t}.cmd")), "@echo off\r\nvtok auto -- %~n0 %*\r\n").map_err(|e| e.to_string())?;
        n += 1;
    }
    Ok(n)
}

pub fn shims_cmd(args: &[String]) -> i32 {
    let dir = args.iter().skip(1).find(|a| !a.starts_with('-')).map(PathBuf::from).unwrap_or_else(default_dir);
    match args.first().map(|s| s.as_str()) {
        Some("install") => match install(&dir) {
            Ok(n) => {
                println!("{n} shims in {}\nPut it first on PATH (any shell or agent launcher), e.g.\n  export PATH=\"{}:$PATH\"\nRemove the directory from PATH (or delete it) to undo. VTOK_OFF=1 turns the shims into plain passthrough.", dir.display(), dir.display());
                0
            }
            Err(e) => { eprintln!("{e}"); 1 }
        },
        Some("list") => { println!("{}", TOOLS.join(" ")); 0 }
        _ => { eprintln!("usage: vtok shims install [DIR] | list   (default DIR: ~/.vtok/shims)"); 2 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_writes_shims_and_path_skips_them() {
        let d = std::env::temp_dir().join(format!("vtok-shims-{}", std::process::id()));
        let real = std::env::temp_dir().join(format!("vtok-real-{}", std::process::id()));
        std::fs::create_dir_all(&real).unwrap();
        std::fs::write(real.join("cargo"), "#!/bin/sh\necho real cargo\n").unwrap();
        assert_eq!(install(&d).unwrap(), TOOLS.len());
        let p = std::env::join_paths([d.clone(), real.clone()]).unwrap();
        let out = path_without_shims("cargo", &p);
        let parts: Vec<PathBuf> = std::env::split_paths(&out).collect();
        assert_eq!(parts, vec![real.clone()]);
        assert!(std::fs::read_to_string(d.join("npm")).unwrap().contains("vtok auto"));
        std::fs::remove_dir_all(&d).ok();
        std::fs::remove_dir_all(&real).ok();
    }
}
