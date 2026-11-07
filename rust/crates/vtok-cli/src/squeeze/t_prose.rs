//! One line of prose: word boundaries and the capital a removed opener hands on.
use super::prose::prose_line;

#[test]
fn word_boundaries_and_capitals() {
    assert_eq!(prose_line("It is a justification, not just a rule."), "It is a justification, not a rule.");
    assert_eq!(prose_line("very good"), "Good");
    assert_eq!(prose_line("we utilize it"), "we use it");
}
