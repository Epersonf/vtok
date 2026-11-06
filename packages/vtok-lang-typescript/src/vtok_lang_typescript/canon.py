"""Canonical structural dump of a tree-sitter tree, for roundtrip equivalence.
Ignores comments, ';' ',' , empty statements; unwraps single-statement bodies; normalizes
newlines inside template literals (they are emitted as \\n escapes)."""

BODY_PARENTS = {"if_statement", "else_clause", "for_statement", "for_in_statement",
                "while_statement", "do_statement"}
_SKIP = {"comment", "empty_statement"}

def canon(node, out=None):
    out = [] if out is None else out
    t = node.type
    if t in _SKIP:
        return out
    if not node.is_named:
        if t not in (";", ","):
            out.append(t)
        return out
    if t == "statement_block" and node.parent is not None and node.parent.type in BODY_PARENTS:
        kids = [c for c in node.named_children if c.type not in _SKIP]
        if len(kids) == 1:
            return canon(kids[0], out)
    if t == "template_string":
        out.append("(template")
        buf = []
        def flush():
            if buf:
                out.append("txt:" + "".join(buf)); buf.clear()
        for c in node.children:
            if c.type in ("string_fragment", "escape_sequence"):
                buf.append(c.text.decode().replace("\n", "\\n"))
            elif c.type == "template_substitution":
                flush(); canon(c, out)
        flush(); out.append(")")
        return out
    if node.child_count == 0 or t in ("string", "regex"):
        out.append(f"{t}:{node.text.decode()}")
        return out
    out.append("(" + t)
    for c in node.children:
        canon(c, out)
    out.append(")")
    return out
