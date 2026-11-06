"""Vtok -> Python source. Deterministic; errors carry line numbers for the retry loop."""
import ast, keyword
from .grammar import STMT_INV, HEADERS, INLINE_BODY, EXPR_INV, TYPES_INV, BITWISE_INV
from .lexer import lex
from vtok.errors import VtokError
from .aliases import rename, restore_imports

ENDERS_OP = {")", "]", "}", "..."}
BODY_KW = INLINE_BODY
PYKW = set(keyword.kwlist)
STARTERS = {"await", "yield", "del", "global", "nonlocal"}
INLINE_OK = HEADERS - {"C", "f"}
# a would-be keyword followed by one of these is really a variable name (`p=1`, `r.x`, `t,u=1,2`)
ASSIGN_LIKE = {"=", "+=", "-=", "*=", "/=", "//=", "%=", "**=", ">>=", "<<=", "||=", "&&=", "@=", "^=", ".", ",", ":"}

def _is_keyword(first, toks):
    """Is the leading single-letter token a Vtok keyword (vs. an identifier the model chose)?"""
    if first == "?":
        return True
    nxt = toks[1] if len(toks) > 1 else None
    if first == "M":   # `from x import y` / relative `from . import y`
        return nxt is not None and (nxt[0] == "NAME" or nxt[1] in (".", "..."))
    if first in ("C", "f", "m"):
        return nxt is not None and nxt[0] == "NAME"
    if first in ("p", "B", "K"):
        return nxt is None
    if first in ("E", "t", "y"):
        return nxt is None or nxt[0] == "NAME"
    return nxt is None or not (nxt[0] == "OP" and nxt[1] in ASSIGN_LIKE)

def _is_end(tok):
    return (tok[0] == "NAME" and tok[1] not in PYKW) or tok[0] in ("NUM", "STR") \
        or (tok[0] == "OP" and tok[1] in ENDERS_OP)

def _is_stmt_start(tok):
    return tok[0] == "NAME" and (tok[1] not in PYKW or tok[1] in STARTERS)

def _is_ender(tok):
    return tok[0] in ("NAME", "NUM", "STR") or (tok[0] == "OP" and tok[1] in ENDERS_OP)

def _expr_to_py(toks):
    out, depth = [], 0
    for i, (k, v) in enumerate(toks):
        prev = toks[i - 1] if i else None
        nxt = toks[i + 1] if i + 1 < len(toks) else None
        if k == "OP" and v in "([{": depth += 1
        elif k == "OP" and v in ")]}": depth -= 1
        if prev == ("OP", "."):
            out.append(v); continue
        if k == "OP" and v in BITWISE_INV:
            out.append(BITWISE_INV[v]); continue
        if v in EXPR_INV and k in ("OP", "NAME"):
            if k == "NAME" and depth > 0 and nxt == ("OP", "="):
                out.append(v); continue
            out.append(EXPR_INV[v]); continue
        if k == "OP" and v in ("?",):
            raise VtokError("unexpected '?' inside expression")
        out.append(v)
    return " ".join(out)

def _line_to_py(text, lineno):
    toks = lex(text, lineno)
    if not toks:
        return ""
    prefix = ""
    if toks[0] == ("NAME", "async") and len(toks) > 1:
        prefix, toks = "async ", toks[1:]
    if len(toks) > 2 and toks[0] == ("NAME", "E") and toks[1] == ("OP", "?"):
        toks = [("NAME", "e")] + toks[2:]       # `E ?c` is just `e c`
    first = toks[0][1]
    if first in STMT_INV and toks[0][0] in ("NAME", "OP") and _is_keyword(first, toks):
        kw = STMT_INV[first]
        rest = toks[1:]
        # split inline body: the header expression ends where a simple statement starts
        depth, split = 0, None
        for i, t in enumerate(rest):
            if t[0] == "OP" and t[1] in "([{": depth += 1
            elif t[0] == "OP" and t[1] in ")]}": depth -= 1
            elif depth == 0 and first in INLINE_OK:
                if i == 0:
                    if first in ("E", "t", "y") or (first == "x" and t[0] == "NAME" and t[1] in BODY_KW):
                        split = 0; break
                    continue
                if _is_stmt_start(t) and _is_end(rest[i - 1]):
                    split = i; break
        head_toks, body_toks = (rest[:split], rest[split:]) if split is not None else (rest, [])
        s = prefix + kw
        e = _expr_to_py(head_toks)
        if e: s += " " + e
        if first in HEADERS: s += ":"
        if body_toks:
            s += " " + _line_to_py(_retext(body_toks), lineno).lstrip()
        return s
    return prefix + _expr_to_py(toks)

def _retext(toks):
    # rebuild text for recursive single-line body (first token is a keyword)
    return " ".join(v for _, v in toks)

def decode_to_python(vtok, lineno_offset=0):
    out = []
    for n, raw in enumerate(vtok.splitlines(), 1):
        if not raw.strip():
            out.append(""); continue
        stripped = raw.lstrip(" ")
        ind = len(raw) - len(stripped)
        try:
            out.append(" " * ind + _line_to_py(stripped, n))
        except VtokError as e:
            raise e if e.line else VtokError(str(e), n)
    return "\n".join(out) + "\n"

def _expand_annotations(tree):
    def fix(ann):
        for n in ast.walk(ann):
            if isinstance(n, ast.Name) and n.id in TYPES_INV:
                n.id = TYPES_INV[n.id]
    for n in ast.walk(tree):
        if isinstance(n, ast.arg) and n.annotation: fix(n.annotation)
        elif isinstance(n, (ast.FunctionDef, ast.AsyncFunctionDef)) and n.returns: fix(n.returns)
        elif isinstance(n, ast.AnnAssign): fix(n.annotation)

def _restore_literals(tree, literals):
    for n in ast.walk(tree):
        if isinstance(n, ast.Constant) and isinstance(n.value, str) and n.value in literals:
            n.value = literals[n.value]

def decode(vtok, table=None, style="native", literals=None):
    """table: {alias: original}. Raises VtokError with line info on malformed input."""
    if style == "native":
        from .native import parse
        tree = parse(vtok)
        if table:
            rename(tree, table)
        if literals:
            _restore_literals(tree, literals)
        return ast.unparse(tree) + "\n"
    py = decode_to_python(vtok)
    try:
        tree = ast.parse(py)
    except SyntaxError as e:
        raise VtokError(f"{e.msg} (python: {py.splitlines()[e.lineno-1].strip()!r})", e.lineno)
    _expand_annotations(tree)
    forced = []
    if table:
        forced = [n for n in ast.walk(tree) if isinstance(n, ast.alias) and n.asname in table]
        rename(tree, table)
    restore_imports(tree, forced)
    return ast.unparse(tree) + "\n"
