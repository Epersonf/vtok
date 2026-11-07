//! Which commands are safe to rewrite for agents whose hook protocol makes a rewrite imply "allow" (Cursor, Codex,
//! Gemini CLI, Antigravity). Only read-only or build-and-test commands qualify; anything that installs, pushes, deploys,
//! deletes or runs an arbitrary target is left to the agent's own approval flow.

fn words(seg: &str) -> Vec<&str> {
    seg.split_whitespace().skip_while(|w| w.contains('=') && !w.starts_with('-') && !w.contains('/')).collect()
}

pub fn low_risk(seg: &str) -> bool {
    let w = words(seg);
    let Some(&p) = w.first() else { return false };
    let prog = p.rsplit('/').next().unwrap_or(p);
    let sub = w.get(1).copied().unwrap_or("");
    let third = w.get(2).copied().unwrap_or("");
    match prog {
        "pytest" | "py.test" | "jest" | "vitest" | "mocha" | "tsc" | "eslint" | "ruff" | "mypy" | "pylint" | "flake8" | "rspec" | "phpunit" | "golangci-lint" | "rubocop" | "pyright" | "ctest" | "ls" | "tree" | "find" | "rg" | "grep" => !w.iter().any(|a| matches!(*a, "--fix" | "--write" | "-delete" | "-exec")),
        "python" | "python3" => sub == "-m" && matches!(third, "pytest" | "unittest" | "mypy"),
        "cargo" => matches!(sub, "test" | "check" | "clippy" | "build" | "doc" | "bench" | "nextest" | "tree"),
        "npm" | "pnpm" | "yarn" | "bun" => matches!(sub, "test" | "lint" | "outdated" | "ls" | "list" | "audit") || (sub == "run" && matches!(third, "test" | "build" | "lint" | "typecheck")),
        "npx" | "bunx" => matches!(sub, "jest" | "vitest" | "tsc" | "eslint" | "mocha"),
        "go" => matches!(sub, "test" | "vet" | "build"),
        "dotnet" | "mvn" | "gradle" | "gradlew" => matches!(sub, "test" | "build" | "check" | "verify" | "compile"),
        "git" => matches!(sub, "status" | "log" | "diff" | "show" | "blame"),
        "gh" => matches!(sub, "pr" | "issue" | "run") && matches!(third, "list" | "view" | "checks" | "diff"),
        "docker" | "podman" => matches!(sub, "ps" | "images" | "logs"),
        "kubectl" => matches!(sub, "get" | "describe" | "logs"),
        "terraform" | "tofu" => matches!(sub, "plan" | "validate"),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_read_and_test_commands_qualify() {
        for yes in ["cargo test", "pytest -q tests", "git status", "npm run lint", "kubectl get pods", "go vet ./...", "ls -R src"] { assert!(low_risk(yes), "{yes}"); }
        for no in ["npm install", "git push", "cargo install x", "terraform apply", "docker compose up", "make deploy", "eslint --fix .", "find . -delete", "pip install x", "kubectl delete pod x"] { assert!(!low_risk(no), "{no}"); }
    }
}
