"""Python source -> Vtok (M0, ast + line transform)."""
import ast
from .grammar import STMT, HEADERS, INLINE_BODY, EXPR, TYPES, BITWISE
from .lexer import lex, wordlike
from vtok.errors import VtokError
from .aliases import build_table, rename, prepare_forced_imports

def strip_docstrings(tree):
    for n in ast.walk(tree):
        if isinstance(n, (ast.Module, ast.ClassDef, ast.FunctionDef, ast.AsyncFunctionDef)):
            b = n.body
            if b and isinstance(b[0], ast.Expr) and isinstance(getattr(b[0], "value", None), ast.Constant) \
               and isinstance(b[0].value.value, str):
                n.body = b[1:] or [ast.Pass()]
    return tree

def _abbrev_annotations(tree):
    def fix(ann):
        for n in ast.walk(ann):
            if isinstance(n, ast.Name) and n.id in TYPES:
                n.id = TYPES[n.id]
    for n in ast.walk(tree):
        if isinstance(n, ast.arg) and n.annotation: fix(n.annotation)
        elif isinstance(n, (ast.FunctionDef, ast.AsyncFunctionDef)) and n.returns: fix(n.returns)
        elif isinstance(n, ast.AnnAssign): fix(n.annotation)

def _join(toks):
    out, prev = [], None
    for t in toks:
        if prev is not None and ((wordlike(prev) and wordlike(t)) or (prev[0] == "NUM" and prev[1].isdigit() and t == ("OP", "."))):
            out.append(" ")
        out.append(t[1])
        prev = t
    return "".join(out)

def _line_to_vtok(text):
    toks = lex(text)
    prefix = []
    if toks and toks[0] == ("NAME", "async") and len(toks) > 1:
        prefix, toks = [("NAME", "async")], toks[1:]
    kw = None
    if toks and toks[0][0] == "NAME" and toks[0][1] in STMT:
        kw = STMT[toks[0][1]]
        toks = toks[1:]
    if kw in HEADERS and toks and toks[-1] == ("OP", ":"):
        toks = toks[:-1]
    # expression-level mapping
    depth, mapped, prev = 0, [], None
    for i, (k, v) in enumerate(toks):
        if v in "([{" and k == "OP": depth += 1
        elif v in ")]}" and k == "OP": depth -= 1
        if k == "OP" and v in BITWISE:
            v = BITWISE[v]
        elif k == "NAME" and v in EXPR and not (prev == ("OP", ".")):
            nxt = toks[i + 1] if i + 1 < len(toks) else None
            if not (depth > 0 and nxt == ("OP", "=")):  # kwarg named like a literal
                k, v = "OP" if EXPR[v] in "!&|" else "NAME", EXPR[v]
        mapped.append((k, v)); prev = (toks[i])
    head = _join(prefix + ([("NAME", kw)] if kw and kw not in "?" else []))
    body = _join(mapped)
    if kw == "?":
        return "?" + body
    if kw is None:
        return (head + " " if head else "") + body
    return head + (" " + body if body else "")

def python_to_vtok_lines(py_text):
    lines = [l for l in py_text.splitlines() if l.strip()]
    parsed = []
    for ln in lines:
        stripped = ln.lstrip(" ")
        ind = (len(ln) - len(stripped)) // 4
        parsed.append((ind, _line_to_vtok(stripped)))
    # inline single-statement bodies
    out, i = [], 0
    while i < len(parsed):
        ind, txt = parsed[i]
        first = txt.split(" ", 1)[0] if not txt.startswith("?") else "?"
        is_header = first in HEADERS and first != "C" and first != "f" and not txt.startswith("async")
        if is_header and i + 1 < len(parsed):
            bi, bt = parsed[i + 1]
            nxt_ind = parsed[i + 2][0] if i + 2 < len(parsed) else 0
            bfirst = bt.split(" ", 1)[0]
            if bi == ind + 1 and nxt_ind <= ind and bfirst in INLINE_BODY:
                out.append(" " * ind + txt + " " + bt)
                i += 2
                continue
        out.append(" " * ind + txt)
        i += 1
    return out

def lean(tree):
    """Read-only view: drop annotations."""
    for n in ast.walk(tree):
        if isinstance(n, ast.arg): n.annotation = None
        elif isinstance(n, (ast.FunctionDef, ast.AsyncFunctionDef)): n.returns = None
        for f in ("body", "orelse", "finalbody"):
            lst = getattr(n, f, None)
            if isinstance(lst, list):
                new = []
                for s in lst:
                    if isinstance(s, ast.AnnAssign):
                        s = ast.Assign(targets=[s.target], value=s.value) if s.value is not None else ast.Expr(s.target)
                    new.append(s)
                setattr(n, f, new)
    return tree

def hide_literals(tree, minimum, out):
    """Replace long plain string constants by '$N' placeholders; out: {placeholder: value}."""
    from vtok.tokens import ntok
    inside = {id(c) for n in ast.walk(tree) if isinstance(n, ast.JoinedStr) for c in ast.walk(n)}
    existing = {n.value for n in ast.walk(tree) if isinstance(n, ast.Constant) and isinstance(n.value, str)}
    by_value, k = {}, 0
    for n in ast.walk(tree):
        if isinstance(n, ast.Constant) and isinstance(n.value, str) and id(n) not in inside and ntok(repr(n.value)) >= minimum:
            if n.value not in by_value:
                k += 1
                while f"${k}" in existing: k += 1
                by_value[n.value] = f"${k}"
                out[f"${k}"] = n.value
            n.value = by_value[n.value]

def skeletonize(tree):
    for n in ast.walk(tree):
        if isinstance(n, (ast.FunctionDef, ast.AsyncFunctionDef)):
            n.body = [ast.Expr(ast.Constant(Ellipsis))]
    return tree

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
