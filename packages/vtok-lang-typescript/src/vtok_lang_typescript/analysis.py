"""Name census of a module: how often each identifier occurs, which names must keep their spelling, which must not be reused."""
from .grammar import KW, KW_INV
from .nodes import IDENT, JS_WORDS, LIKE_IDENT, text

def _excluded_and_counts(root):
    counts, excl, avoid = {}, set(), set(JS_WORDS) | set(KW_INV) | set(KW.values())
    def idents(n):
        if n.type in IDENT: yield text(n)
        for c in n.children: yield from idents(c)
    stack = [root]
    while stack:
        n = stack.pop(); stack.extend(n.children)
        t = n.type
        if t in LIKE_IDENT: avoid.add(text(n))
        if t in IDENT:
            s = text(n); counts[s] = counts.get(s, 0) + 1
        elif t in ("shorthand_property_identifier", "shorthand_property_identifier_pattern"):
            excl.add(text(n))
        elif t == "import_statement":
            excl.update(idents(n))
        elif t == "export_statement":
            for c in n.children:
                if c.type in IDENT: excl.add(text(c))
                elif c.type == "export_clause": excl.update(idents(c))
            d = n.child_by_field_name("declaration")
            if d is not None:
                nm = d.child_by_field_name("name")
                if nm is not None: excl.update(idents(nm))
                for vd in d.named_children:
                    if vd.type == "variable_declarator":
                        excl.update(idents(vd.child_by_field_name("name")))
    return counts, excl, avoid
