"""Python source -> Vtok (M0, ast + line transform). The tree views live in `views`, the legacy line style in `legacy`."""
import ast
from vtok.errors import VtokError
from .aliases import build_table, rename, prepare_forced_imports
from .legacy import _abbrev_annotations, python_to_vtok_lines
from .views import hide_literals, lean, skeletonize, strip_docstrings

def encode(source, alias=True, verify=True, extra_avoid=(), table=None, view="full", style="native", hide=0, lits_out=None):
    """Return (vtok_text, table{orig:alias})."""
    tree = strip_docstrings(ast.parse(source))
    if view == "skeleton":
        skeletonize(tree)
    elif view == "lean":
        lean(tree); ast.fix_missing_locations(tree)
    ref = ast.dump(tree)
    lits = {}
    if hide:
        hide_literals(tree, hide, lits)
        if lits_out is not None: lits_out.update(lits)
    if style == "legacy":
        prepare_forced_imports(tree)
    if table is None:
        table = build_table(tree, extra_avoid) if alias else {}
    else:
        table = {k: v for k, v in table.items()}
    if table:
        rename(tree, table)
    if style == "legacy":
        _abbrev_annotations(tree)
    py = ast.unparse(tree)
    if style == "native":
        from .native import render
        vt = render(py)
    else:
        vt = "\n".join(python_to_vtok_lines(py)) + "\n"
    if verify:
        from .decode import decode
        back = decode(vt, {a: o for o, a in table.items()}, style=style, literals=lits)
        if ast.dump(ast.parse(back)) != ref:
            raise VtokError("roundtrip verification failed (AST differs)")
    return vt, table
