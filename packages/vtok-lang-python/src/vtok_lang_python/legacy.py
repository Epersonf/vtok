"""Legacy style (`style="legacy"`): the unparsed source is mapped line by line, with one-character statement keywords."""
import ast
from .grammar import STMT, HEADERS, INLINE_BODY, EXPR, TYPES, BITWISE
from .lexer import lex, wordlike

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
