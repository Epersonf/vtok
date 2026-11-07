//! A member chunk (a method taken out of its class) is parsed inside a synthetic class so that it is valid Python.
use crate::tree::{named_children, trivia};
use tree_sitter::Node;

const WRAP: &str = "__vtok_wrap__";

pub(super) fn wrapped(text: &str, member: bool) -> String {
    if member { format!("class {WRAP}:\n{text}") } else { text.to_string() }
}

/// Statements of the module, or of the synthetic wrapper class when `member`.
pub(super) fn body_of<'a>(root: Node<'a>, member: bool) -> Vec<Node<'a>> {
    if member {
        named_children(root).into_iter().find(|c| !trivia(*c))
            .and_then(|c| c.child_by_field_name("body"))
            .map(|b| named_children(b).into_iter().filter(|x| !trivia(*x)).collect())
            .unwrap_or_default()
    } else {
        named_children(root).into_iter().filter(|x| !trivia(*x)).collect()
    }
}
