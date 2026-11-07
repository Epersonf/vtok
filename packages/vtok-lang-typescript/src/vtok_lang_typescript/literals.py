"""Long string literals that are replaced by short placeholders (`"$1"`, `"$2"`, ...)."""
from .nodes import text

_NO_HIDE_PARENTS = {"import_statement", "export_statement", "expression_statement", "literal_type", "import_require_clause"}
_NO_HIDE_ANCESTORS = {"type_annotation", "type_alias_declaration", "interface_declaration", "literal_type", "import_statement", "export_statement", "ambient_declaration"}

def collect_literals(root, minimum):
    """-> ({start_byte: placeholder}, {placeholder: original text}) for long, non-type strings."""
    from vtok.tokens import ntok
    strings, existing, st = [], set(), [root]
    while st:
        n = st.pop(); st.extend(n.children)
        if n.type == "string":
            existing.add(text(n))
            p, ok = n.parent, True
            while p is not None:
                if p.type in _NO_HIDE_ANCESTORS and not (p.type == "export_statement" and n.parent is not p):
                    ok = False; break
                p = p.parent
            if ok and n.parent.type not in _NO_HIDE_PARENTS and ntok(text(n)) >= minimum:
                strings.append(n)
    pos, back, k = {}, {}, 0
    by_text = {}
    for n in sorted(strings, key=lambda x: x.start_byte):
        s = text(n)
        if s not in by_text:
            k += 1
            while f'"${k}"' in existing: k += 1
            by_text[s] = f'"${k}"'
            back[by_text[s]] = s
        pos[n.start_byte] = by_text[s]
    return pos, back
