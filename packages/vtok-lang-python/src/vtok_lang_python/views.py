"""Tree views and rewrites applied before the notation is made: docstrings out, annotations out (lean), long strings hidden, bodies out (skeleton)."""
import ast

def strip_docstrings(tree):
    for n in ast.walk(tree):
        if isinstance(n, (ast.Module, ast.ClassDef, ast.FunctionDef, ast.AsyncFunctionDef)):
            b = n.body
            if b and isinstance(b[0], ast.Expr) and isinstance(getattr(b[0], "value", None), ast.Constant) \
               and isinstance(b[0].value.value, str):
                n.body = b[1:] or [ast.Pass()]
    return tree

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
