//! Dedicated views for build/test tools (cargo, go, golangci-lint, mvn, gradle, sbt, dotnet, msbuild, ctest,
//! make/cmake/gcc/clang). The tool is decided from the command line; each child module owns one tool family.
//! Contract: `view(cmd, lines)` returns Some(compact text) only when the output looks like what that tool prints
//! and the result is smaller than the input; otherwise None so the generic summariser runs.
mod cargo;
mod cc;
mod ctest;
mod dotnet;
mod go;
mod golint;
mod gradle;
mod mvn;
mod sbt;
#[cfg(test)] mod t_cargo;
#[cfg(test)] mod t_dotnet;
#[cfg(test)] mod t_go;
#[cfg(test)] mod t_gradle_sbt;
#[cfg(test)] mod t_mvn;
#[cfg(test)] mod t_native;

use regex::Regex;

pub fn view(cmd: &str, lines: &[String]) -> Option<String> {
    if lines.iter().all(|l| l.trim().is_empty()) { return None; }
    for (tool, args) in parse(cmd).into_iter().rev() {
        let sub = args.iter().find(|a| !a.starts_with('-') && !a.starts_with('+')).map(String::as_str).unwrap_or("");
        let r = match tool.as_str() {
            "cargo" | "cross" => cargo::view(sub, lines),
            "go" => go::view(sub, &args, lines),
            "golangci-lint" => golint::view(lines),
            "mvn" => mvn::view(lines),
            "gradle" => gradle::view(lines),
            "sbt" => sbt::view(lines),
            "dotnet" => dotnet::view(sub, lines),
            "msbuild" | "xbuild" => dotnet::view("build", lines),
            "ctest" => ctest::view(lines),
            "make" | "gmake" | "mingw32-make" | "ninja" | "cmake" | "gcc" | "g++" | "cc" | "c++" | "clang" | "clang++" => cc::view(lines),
            _ => None,
        };
        if r.is_some() { return r; }
    }
    None
}

/// Tools in the command line, one per `&&`/`;`/`|` segment: (normalised name, args). Skips env vars and wrappers.
fn parse(cmd: &str) -> Vec<(String, Vec<String>)> {
    let sep = Regex::new(r"&&|\|\||;|\|").unwrap();
    let env = Regex::new(r"^[A-Za-z_][A-Za-z0-9_]*=").unwrap();
    sep.split(cmd).filter_map(|seg| {
        let mut it = seg.split_whitespace()
            .skip_while(|w| env.is_match(w) || matches!(*w, "sudo" | "time" | "env" | "nice" | "command" | "exec" | "call"))
            .map(|w| w.trim_matches(|c| c == '"' || c == '\''));
        let mut name = it.next()?.rsplit(['/', '\\']).next()?.to_lowercase();
        for ext in [".exe", ".cmd", ".bat", ".sh", ".ps1"] { if let Some(s) = name.strip_suffix(ext) { name = s.to_string(); } }
        let name = match name.as_str() { "gradlew" => "gradle".into(), "mvnw" => "mvn".into(), _ => name };
        Some((name, it.map(String::from).collect()))
    }).collect()
}

/// Join the kept lines; None when empty or not smaller than the input.
fn finish(out: Vec<String>, input: &[String]) -> Option<String> {
    if out.is_empty() { return None; }
    let s = out.join("\n") + "\n";
    let before: usize = input.iter().map(|l| l.len() + 1).sum();
    (s.len() < before).then_some(s)
}

fn trunc(s: &str, n: usize) -> String {
    if s.chars().count() <= n { s.to_string() } else { s.chars().take(n).collect::<String>() + "..." }
}

/// Remove repeated lines, keeping the first occurrence.
fn uniq(v: &mut Vec<String>) {
    let mut seen = std::collections::HashSet::new();
    v.retain(|x| seen.insert(x.clone()));
}

/// Push at most `n` items, then a "... +k more" marker.
fn cap(out: &mut Vec<String>, items: &[String], n: usize) {
    out.extend(items.iter().take(n).cloned());
    if items.len() > n { out.push(format!("... +{} more", items.len() - n)); }
}

/// Repeated diagnostics collapsed by key: (key, count, first location).
#[derive(Default)]
struct Groups(Vec<(String, usize, String)>);
impl Groups {
    fn add(&mut self, key: &str, loc: &str) {
        match self.0.iter_mut().find(|g| g.0 == key) {
            Some(g) => g.1 += 1,
            None => self.0.push((key.to_string(), 1, loc.to_string())),
        }
    }
    fn total(&self) -> usize { self.0.iter().map(|g| g.1).sum() }
    fn render(&self, out: &mut Vec<String>) {
        for (k, n, loc) in &self.0 {
            let c = if *n > 1 { format!(" (x{n})") } else { String::new() };
            out.push(if loc.is_empty() { format!("{k}{c}") } else { format!("{k}{c} @ {loc}") });
        }
    }
}

#[cfg(test)]
fn ls(s: &str) -> Vec<String> { s.lines().map(String::from).collect() }

/// Test helper: run the view, assert it produced something smaller than the input.
#[cfg(test)]
fn run(cmd: &str, input: &str) -> String {
    let out = view(cmd, &ls(input)).expect("view returned None");
    assert!(out.len() < input.len(), "not smaller:\n{out}");
    assert!(out.ends_with('\n'));
    out
}
