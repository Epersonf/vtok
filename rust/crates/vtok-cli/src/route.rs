//! Model routing: pick the cheapest model tier likely to do a request well. Deterministic and local: a score from
//! what the request asks for (size, keywords, code, number of files) maps to a tier (small / medium / large) and a
//! reasoning effort. Tier -> model comes from the user config (`"models"`), then `VTOK_MODEL_SMALL|MEDIUM|LARGE`, then defaults.
//! Used by `vtok route`, the `vtok_route` MCP tool and `vtok proxy` (for requests whose model is "auto").
use serde_json::{json, Value};

const LARGE: &[&str] = &["architecture", "architect", "design", "refactor", "migrate", "migration", "race condition", "deadlock", "security", "vulnerab", "performance", "optimi", "root cause", "investigate", "algorithm", "concurren", "distributed", "rewrite", "from scratch", "trade-off", "tradeoff", "prove", "why does", "why is"];
const SMALL: &[&str] = &["typo", "rename", "format", "lint", "comment", "docstring", "translate", "what does this line", "list the", "show me", "add import", "bump", "spelling", "one-liner", "summarize this", "what is "];
const DEEP: &[&str] = &["step by step", "think hard", "carefully", "thoroughly", "ultrathink"];

pub struct Route { pub tier: &'static str, pub effort: &'static str, pub score: i32, pub model: String, pub reasons: Vec<String> }

fn count(low: &str, words: &[&str]) -> usize { words.iter().filter(|w| low.contains(**w)).count() }

pub fn classify(prompt: &str, files: usize) -> Route {
    let low = prompt.to_lowercase();
    let mut reasons = vec![];
    let mut s = 0i32;
    let n = prompt.chars().count();
    if n > 1500 { s += 3; reasons.push("long request".to_string()); } else if n > 500 { s += 2; } else if n >= 80 { s += 1; } else { s -= 1; reasons.push("short request".to_string()); }
    let big = count(&low, LARGE).min(2) as i32;
    if big > 0 { s += big * 2; reasons.push(format!("{big} hard-task keyword(s)")); }
    let small = count(&low, SMALL).min(2) as i32;
    if small > 0 { s -= small * 2; reasons.push(format!("{small} simple-task keyword(s)")); }
    let fences = (prompt.matches("```").count() / 2).min(2) as i32;
    s += fences;
    let paths = prompt.split_whitespace().filter(|w| w.contains('/') && w.contains('.')).count().max(files);
    if paths >= 3 { s += 1; reasons.push(format!("{paths} files involved")); }
    if low.contains("all files") || low.contains("entire") || low.contains("across the") { s += 1; reasons.push("wide scope".into()); }
    let tier = if s <= 0 { "small" } else if s <= 3 { "medium" } else { "large" };
    let deep = count(&low, DEEP) > 0;
    let effort = match (tier, deep) { ("small", false) => "low", ("small", true) | ("medium", false) => "medium", _ => "high" };
    Route { tier, effort, score: s, model: model_for(tier), reasons }
}

fn model_for(tier: &str) -> String {
    let cfg = crate::config::user_config_path().and_then(|p| std::fs::read_to_string(p).ok()).and_then(|t| serde_json::from_str::<Value>(&t).ok());
    if let Some(m) = cfg.as_ref().and_then(|c| c.pointer(&format!("/models/{tier}"))).and_then(|m| m.as_str()) { return m.to_string(); }
    if let Ok(m) = std::env::var(format!("VTOK_MODEL_{}", tier.to_uppercase())) { return m; }
    match tier { "small" => "claude-haiku-5-5", "medium" => "claude-sonnet-5-5", _ => "claude-opus-5-5" }.to_string()
}

impl Route {
    pub fn to_json(&self) -> Value { json!({"tier": self.tier, "model": self.model, "effort": self.effort, "score": self.score, "reasons": self.reasons}) }
    pub fn line(&self) -> String { format!("{} model={} effort={} (score {}: {})", self.tier, self.model, self.effort, self.score, if self.reasons.is_empty() { "no strong signals".to_string() } else { self.reasons.join(", ") }) }
}

pub fn run(args: &[String]) -> i32 {
    let files = args.iter().position(|a| a == "--files").and_then(|i| args.get(i + 1)).and_then(|v| v.parse().ok()).unwrap_or(0);
    let text: Vec<&String> = args.iter().enumerate().filter(|(i, a)| !a.starts_with("--") && (*i == 0 || args[i - 1] != "--files")).map(|(_, a)| a).collect();
    if text.is_empty() { eprintln!("usage: vtok route \"request text\" [--files N] [--json]"); return 2; }
    let r = classify(&text.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(" "), files);
    if args.iter().any(|a| a == "--json") { println!("{}", r.to_json()); } else { println!("{}", r.line()); }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tiers_follow_the_request() {
        assert_eq!(classify("fix the typo in the README", 0).tier, "small");
        assert_eq!(classify("add a --verbose flag to the CLI that prints each file as it is processed and update the tests for it accordingly", 0).tier, "medium");
        let hard = classify("Investigate the root cause of this deadlock and refactor the locking design across the whole service, step by step. See src/a/x.rs src/b/y.rs src/c/z.rs", 0);
        assert_eq!((hard.tier, hard.effort), ("large", "high"));
        assert!(classify("rename x", 0).effort == "low" && !hard.reasons.is_empty());
    }

    #[test]
    fn models_come_from_env_then_defaults() {
        std::env::set_var("VTOK_MODEL_SMALL", "my-small");
        assert_eq!(classify("typo", 0).model, "my-small");
        std::env::remove_var("VTOK_MODEL_SMALL");
        assert!(classify("typo", 0).model.contains("haiku"));
    }
}
