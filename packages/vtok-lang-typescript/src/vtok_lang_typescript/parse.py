import tree_sitter_typescript as _tsts
from tree_sitter import Language as _L, Parser as _P

_TS = _L(_tsts.language_typescript())
_TSX = _L(_tsts.language_tsx())

def parse(src: bytes, tsx=False):
    return _P(_TSX if tsx else _TS).parse(src)

def first_error(node):
    """Return the first ERROR/missing node (or None)."""
    if node.type == "ERROR" or node.is_missing:
        return node
    if not node.has_error:
        return None
    for c in node.children:
        e = first_error(c)
        if e is not None:
            return e
    return node
