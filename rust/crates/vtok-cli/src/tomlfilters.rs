//! Declarative command filters in RTK's TOML format (`[filters.NAME]` with `match_command`, `strip_lines_matching`,
//! `max_lines`, ...). The built-in set is `filters/builtin.toml`; the user can add `~/.config/vtok/filters.toml`
//! (same schema, user config only). Pipeline: strip_ansi, replace, match_output, strip/keep lines, truncate_lines_at,
//! head/tail, max_lines, on_empty.
use regex::{Regex, RegexSet};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::sync::OnceLock;

const BUILTIN: &str = include_str!("../filters/builtin.toml");

#[derive(Deserialize, Default)]
struct Rule { pattern: String, replacement: Option<String>, message: Option<String>, unless: Option<String> }

#[derive(Deserialize, Default)]
struct Raw {
    match_command: String,
    #[serde(default)] strip_ansi: bool,
    #[serde(default)] replace: Vec<Rule>,
    #[serde(default)] match_output: Vec<Rule>,
    #[serde(default)] strip_lines_matching: Vec<String>,
    #[serde(default)] keep_lines_matching: Vec<String>,
    truncate_lines_at: Option<usize>,
    head_lines: Option<usize>,
    tail_lines: Option<usize>,
    max_lines: Option<usize>,
    on_empty: Option<String>,
}

#[derive(Deserialize)]
struct File { #[serde(default)] filters: BTreeMap<String, Raw>, #[serde(default)] tests: BTreeMap<String, Vec<Case>> }
// the built-in filters' own test cases and names are only read by the unit test below
#[derive(Deserialize)]
#[cfg_attr(not(test), allow(dead_code))]
pub struct Case { pub name: String, pub input: String, pub expected: String }

#[cfg_attr(not(test), allow(dead_code))]
pub struct Filter {
    pub name: String,
    cmd: Regex,
    raw: Raw,
    replace: Vec<(Regex, String)>,
    outputs: Vec<(Regex, String, Option<Regex>)>,
    strip: Option<RegexSet>,
    keep: Option<RegexSet>,
    /// From the user's own filters file (the built-in ones can be bypassed by a dedicated view, these never are).
    user: bool,
}

fn compile(name: &str, raw: Raw) -> Option<Filter> {
    let cmd = Regex::new(&raw.match_command).ok()?;
    let replace = raw.replace.iter().map(|r| Some((Regex::new(&r.pattern).ok()?, r.replacement.clone().unwrap_or_default()))).collect::<Option<Vec<_>>>()?;
    let outputs = raw.match_output.iter().map(|r| Some((Regex::new(&r.pattern).ok()?, r.message.clone().unwrap_or_default(), match &r.unless { Some(u) => Some(Regex::new(u).ok()?), None => None }))).collect::<Option<Vec<_>>>()?;
    let set = |v: &Vec<String>| if v.is_empty() { Some(None) } else { RegexSet::new(v).ok().map(Some) };
    let (strip, keep) = (set(&raw.strip_lines_matching)?, set(&raw.keep_lines_matching)?);
    Some(Filter { name: name.to_string(), cmd, raw, replace, outputs, strip, keep, user: false })
}

fn parse(text: &str) -> (Vec<Filter>, BTreeMap<String, Vec<Case>>) {
    let Ok(f) = toml::from_str::<File>(text) else { return (vec![], BTreeMap::new()) };
    (f.filters.into_iter().filter_map(|(n, r)| compile(&n, r)).collect(), f.tests)
}

fn builtin() -> &'static Vec<Filter> { static B: OnceLock<Vec<Filter>> = OnceLock::new(); B.get_or_init(|| parse(BUILTIN).0) }

fn user() -> &'static Vec<Filter> {
    static U: OnceLock<Vec<Filter>> = OnceLock::new();
    U.get_or_init(|| {
        let p = crate::userfilters::path().map(|p| p.with_extension("toml"));
        p.and_then(|p| std::fs::read_to_string(p).ok()).map(|t| parse(&t).0.into_iter().map(|f| Filter { user: true, ..f }).collect()).unwrap_or_default()
    })
}

fn strip_ansi(s: &str) -> String {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new("\x1b\\[[0-9;?]*[ -/]*[@-~]").unwrap()).replace_all(s, "").into_owned()
}

impl Filter {
    pub fn apply(&self, input: &str) -> String {
        let mut lines: Vec<String> = input.lines().map(String::from).collect();
        if self.raw.strip_ansi { lines = lines.iter().map(|l| strip_ansi(l)).collect(); }
        if !self.replace.is_empty() { for l in lines.iter_mut() { for (re, to) in &self.replace { *l = re.replace_all(l, to.as_str()).into_owned(); } } }
        if !self.outputs.is_empty() {
            let blob = lines.join("\n");
            for (re, msg, unless) in &self.outputs {
                if re.is_match(&blob) && !unless.as_ref().map_or(false, |u| u.is_match(&blob)) { return msg.clone(); }
            }
        }
        if let Some(s) = &self.strip { lines.retain(|l| !s.is_match(l)); } else if let Some(k) = &self.keep { lines.retain(|l| k.is_match(l)); }
        if let Some(n) = self.raw.truncate_lines_at { for l in lines.iter_mut() { if l.chars().count() > n { *l = l.chars().take(n.saturating_sub(3)).collect::<String>() + "..."; } } }
        let total = lines.len();
        match (self.raw.head_lines, self.raw.tail_lines) {
            (Some(h), Some(t)) if total > h + t => { let mut r = lines[..h].to_vec(); r.push(format!("... ({} lines omitted)", total - h - t)); r.extend_from_slice(&lines[total - t..]); lines = r; }
            (Some(h), None) if total > h => { lines.truncate(h); lines.push(format!("... ({} lines omitted)", total - h)); }
            (None, Some(t)) if total > t => { let o = total - t; lines = lines[o..].to_vec(); lines.insert(0, format!("... ({o} lines omitted)")); }
            _ => {}
        }
        if let Some(m) = self.raw.max_lines { if lines.len() > m { let d = lines.len() - m; lines.truncate(m); lines.push(format!("... ({d} lines truncated)")); } }
        let out = lines.join("\n");
        if out.trim().is_empty() { if let Some(m) = &self.raw.on_empty { return m.clone(); } }
        out
    }
}

fn find(cmd: &str) -> Option<&'static Filter> {
    let c = cmd.trim();
    user().iter().chain(builtin().iter()).find(|f| f.cmd.is_match(c))
}

/// True when a declarative filter exists for this command line (the hook and shims use it to decide what to wrap).
pub fn matches(cmd: &str) -> bool { find(cmd).is_some() }

/// The filtered lines for `cmd` and whether the filter is the user's own, or None when no filter matches.
pub fn apply(cmd: &str, lines: &[String]) -> Option<(Vec<String>, bool)> {
    let f = find(cmd)?;
    Some((f.apply(&lines.join("\n")).lines().map(String::from).collect(), f.user))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_builtin_filter_compiles_and_passes_its_own_tests() {
        let (filters, tests) = parse(BUILTIN);
        assert_eq!(filters.len(), 62, "some filter failed to compile");
        let mut bad = vec![];
        let mut n = 0;
        for (name, cases) in &tests {
            let f = filters.iter().find(|f| &f.name == name).unwrap_or_else(|| panic!("no filter {name}"));
            for c in cases {
                n += 1;
                let got = f.apply(&c.input);
                if got.trim_end() != c.expected.trim_end() { bad.push(format!("{name}/{}: got {:?} want {:?}", c.name, got, c.expected)); }
            }
        }
        assert!(n > 100, "only {n} cases");
        assert!(bad.is_empty(), "{} of {n} failed:\n{}", bad.len(), bad.join("\n"));
    }

    #[test]
    fn command_matching() {
        assert!(matches("terraform plan -out x") && matches("make all") && !matches("echo hi"));
    }
}
