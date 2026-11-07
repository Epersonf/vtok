//! `vtok init --terse`: the optional answer-style note (what the agent writes, not what it reads). Off by default.
//! Measured with benchmarks/style_eval.py (Caveman's method: same prompts through `claude -p`, against a plain
//! "Answer concisely." control). The note is kept short on purpose: it is read on every turn, so each word it adds
//! is input cost that the shorter answers have to pay back. Most of the cut comes from scope (answer what was asked,
//! leave out background and extras), not from dropping grammar; text other people read stays in full sentences.

pub const NOTE: &str = "## Answer style
- Give what the user needs now, not all you know: the direct answer first, then only the reason or step that changes what they do. Leave out background, alternatives, pitfalls, tips and examples unless asked.
- No preamble, no restating the question, no closing summary or offer of more help. No headings, tables or bold in short answers. When changing code, show only the changed lines.
- Keep exact: code, commands, paths, error text, numbers, units, and every not/never/only/except.
- Reply in the user's language; technical terms and code stay as written.
- Text others will read (commit, PR, issue, email): plain full sentences, still short. Commit subject: imperative, under 60 characters.
- Review: only real problems, one line each as `path:line problem -> fix`.
- Before a destructive or irreversible step, say so and name the safeguard (backup, dry run).
";

/// Variants accepted after `--terse` (none besides the default: a longer one did not pay for its input cost).
pub const VARIANTS: &str = "(none)";

/// The note for `--terse [VARIANT]`; None for an unknown variant.
pub fn note(variant: Option<&str>) -> Option<&'static str> {
    match variant { None => Some(NOTE), Some(_) => None }
}

/// Conservative token estimate (about 3 characters per token): Claude Code reported +283 input tokens for this note.
#[cfg(test)]
pub fn approx_tokens(s: &str) -> usize { (s.chars().count() + 2) / 3 }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn note_stays_cheap() {
        // Caveman's SKILL.md is ~4.7k characters (+1716 input tokens per call in the same eval); keep this far below.
        assert!(approx_tokens(NOTE) <= 320, "note grew to ~{} tokens", approx_tokens(NOTE));
        assert!(NOTE.lines().count() <= 9);
    }

    #[test]
    fn note_has_the_key_rules() {
        let n = NOTE.to_lowercase();
        for must in ["direct answer first", "preamble", "summary", "not/never/only/except", "numbers", "user's language",
                     "technical terms and code stay as written", "commit", "imperative", "60 characters",
                     "`path:line problem -> fix`", "irreversible", "full sentences"] {
            assert!(n.contains(must), "note lost the rule {must:?}");
        }
        assert!(NOTE.starts_with("## Answer style\n") && NOTE.ends_with('\n'));
    }

    #[test]
    fn variants() {
        assert_eq!(note(None), Some(NOTE));
        assert_eq!(note(Some("ultra")), None);
    }
}
