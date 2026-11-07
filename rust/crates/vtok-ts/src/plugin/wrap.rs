//! A member chunk (a method taken out of its class) is parsed inside a synthetic class so that it is valid TypeScript.
use crate::tree::named_children;
use tree_sitter::Node;

pub(super) const WRAP: &str = "__vtok_wrap__";

pub(super) fn wrapped(text: &str, member: bool) -> String {
    if member { format!("class {WRAP} {{\n{text}\n}}") } else { text.to_string() }
}

pub(super) fn wrapper_members<'a>(root: Node<'a>) -> Vec<Node<'a>> {
    named_children(root).into_iter().next()
        .and_then(|c| c.child_by_field_name("body"))
        .map(named_children).unwrap_or_default()
}
