//! Views and outline units of the languages that are not code (CSS, HTML, SQL); see `css_view`, `html_view`,
//! `sql_view` for what each view keeps.
use crate::spec::{Mode, Spec};
use crate::tree::*;
use crate::{css_view, html_view, sql_view};
use tree_sitter::Node;
use vtok_core::lang::View;
use vtok_core::language::Unit;

/// The skeleton or lean text of a CSS, HTML or SQL file; None for code languages (their views are in `Emit`).
pub fn view(spec: &'static Spec, source: &str, root: Node, v: View) -> Option<String> {
    let src = source.as_bytes();
    Some(match (spec.mode, spec.stmts, v) {
        (_, _, View::Full) => return None,
        (Mode::Css, _, View::Skeleton) => css_view::skeleton(src, root),
        (Mode::Css, _, View::Lean) => css_view::lean(spec, src, root),
        (Mode::Html, _, View::Skeleton) => html_view::skeleton(src, root),
        (Mode::Html, _, View::Lean) => html_view::lean(spec, src, root),
        (Mode::Code, true, View::Skeleton) => sql_view::skeleton(spec, source),
        (Mode::Code, true, View::Lean) => sql_view::lean(spec, source),
        _ => return None,
    })
}

fn unit(n: Node, name: String) -> Unit {
    Unit { name, kind: n.kind().into(), start: n.start_position().row + 1, end: n.end_position().row + 1, member: false, parent: None, indent: n.start_position().column }
}

fn css_rules(n: Node, src: &[u8], deep: bool, out: &mut Vec<Unit>) {
    for c in named_children(n) {
        if !css_view::is_rule(c) { if c.kind() == "ERROR" && deep { css_rules(c, src, deep, out); } continue; }
        out.push(unit(c, css_view::header(c, src)));
        if deep { for r in css_view::nested(c) { css_rules_one(r, src, out); } }
    }
}

fn css_rules_one(n: Node, src: &[u8], out: &mut Vec<Unit>) {
    out.push(unit(n, css_view::header(n, src)));
    for r in css_view::nested(n) { css_rules_one(r, src, out); }
}

/// Outline (`max_lines <= 1`: every rule / landmark / statement, nested ones too) or chunk units (top-level items).
pub fn units(spec: &Spec, source: &str, root: Node, max_lines: usize) -> Option<Vec<Unit>> {
    let src = source.as_bytes();
    let outline = max_lines <= 1;
    let mut out = vec![];
    match spec.mode {
        Mode::Code if spec.stmts => return Some(sql_view::units(spec, source)),
        Mode::Css => css_rules(root, src, outline, &mut out),
        Mode::Html if outline => {
            let mut l = vec![];
            html_view::landmarks(root, src, &mut l);
            out = l.into_iter().map(|(n, s)| unit(n, s)).collect();
        }
        Mode::Html => for c in named_children(root).into_iter().filter(|c| !is_comment(*c)) {
            let name = if html_view::is_element(c) { html_view::label(c, src) } else { c.kind().to_string() };
            out.push(unit(c, name));
        },
        Mode::Code => return None,
    }
    Some(out)
}
