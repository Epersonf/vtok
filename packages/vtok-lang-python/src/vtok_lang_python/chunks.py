"""Chunking support for the Python plugin."""
import ast
from vtok.language import Unit
from vtok.errors import VtokError
from .encode import strip_docstrings
from .decode import decode as _decode

WRAP = "__vtok_wrap__"

def _label(n):
    if isinstance(n, (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef)):
        return n.name
    if isinstance(n, (ast.Import, ast.ImportFrom)):
        return "imports"
    if isinstance(n, (ast.Assign, ast.AnnAssign, ast.AugAssign)):
        ts = n.targets if isinstance(n, ast.Assign) else [n.target]
        names = [t.id for t in ts if isinstance(t, ast.Name)]
        return ",".join(names) or "assign"
    return "code"

def _rng(n):
    return min([n.lineno] + [d.lineno for d in getattr(n, "decorator_list", [])]), n.end_lineno

def units(source, max_lines):
    out = []
    for n in ast.parse(source).body:
        s, e = _rng(n)
        if isinstance(n, ast.ClassDef) and e - s + 1 > max_lines:
            for i, m in enumerate(n.body):
                if i == 0 and isinstance(m, ast.Expr) and isinstance(getattr(m, "value", None), ast.Constant) \
                   and isinstance(m.value.value, str):
                    continue  # class docstring stays in the shell
                ms, me = _rng(m)
                out.append(Unit(_label(m), type(m).__name__, ms, me, True, n.name, m.col_offset))
        else:
            out.append(Unit(_label(n), type(n).__name__, s, e, False, None, n.col_offset))
    return out

def _wrapped(text, member):
    return f"class {WRAP}:\n{text}" if member else text

def encode_chunk(lang, text, table, member):
    if member:
        vt = lang.encode(_wrapped(text, True), table=table).vtok
        return "".join(l[1:] + "\n" for l in vt.splitlines()[1:])
    return lang.encode(text, table=table).vtok

def _is_doc(b):
    return b and isinstance(b[0], ast.Expr) and isinstance(getattr(b[0], "value", None), ast.Constant) \
        and isinstance(b[0].value.value, str)

def _docs(tree):
    out, seen = {}, {}
    if _is_doc(tree.body) and not (len(tree.body) == 1 and isinstance(tree.body[0], ast.ClassDef)):
        out[""] = (tree.body[0], len(tree.body) == 1)
    def walk(node, prefix):
        for n in ast.iter_child_nodes(node):
            if isinstance(n, (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef)):
                q = prefix + n.name
                k = seen[q] = seen.get(q, -1) + 1
                b = n.body
                if _is_doc(b):
                    out[(q, k)] = (b[0], len(b) == 1)
                walk(n, q + ".")
            else:
                walk(n, prefix)
    walk(tree, "")
    return out

def _reattach(new_tree, orig_tree):
    docs, seen = _docs(orig_tree), {}
    if "" in docs and not _is_doc(new_tree.body):
        doc, only_doc = docs[""]
        if only_doc and len(new_tree.body) == 1 and isinstance(new_tree.body[0], ast.Pass):
            new_tree.body = [doc]
        else:
            new_tree.body.insert(0, doc)
    def walk(node, prefix):
        for n in ast.iter_child_nodes(node):
            if isinstance(n, (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef)):
                q = prefix + n.name
                k = seen[q] = seen.get(q, -1) + 1
                b = n.body
                if (q, k) in docs and not _is_doc(b):
                    doc, only_doc = docs[(q, k)]
                    n.body = [doc] + ([] if only_doc and len(b) == 1 and isinstance(b[0], ast.Pass) else b)
                walk(n, q + ".")
            else:
                walk(n, prefix)
    walk(new_tree, "")

def _body_module(tree, member):
    """Module of the statements to splice (strip the synthetic wrapper class)."""
    if member:
        return ast.Module(body=tree.body[0].body, type_ignores=[])
    return tree

def _indent(text, pad):
    """Indent code lines by `pad`, leaving lines inside multi-line string literals untouched."""
    import io, tokenize
    inside = set()
    try:
        for tok in tokenize.generate_tokens(io.StringIO(text).readline):
            if tok.type == tokenize.STRING and tok.end[0] > tok.start[0]:
                inside.update(range(tok.start[0] + 1, tok.end[0] + 1))
    except (tokenize.TokenError, IndentationError):
        pass
    return "".join((l if (i in inside or not l.strip()) else pad + l)
                   for i, l in enumerate(text.splitlines(keepends=True), 1))

def decode_chunk(vtok, inv, member, indent, original=None):
    v = (f"class {WRAP}:\n" + "".join(" " + l + "\n" for l in vtok.splitlines())) if member else vtok
    tree = ast.parse(_decode(v, inv))
    mod = _body_module(tree, member)
    if original is not None:
        try:
            _reattach(mod, _body_module(ast.parse(_wrapped(original, member)), member))
        except SyntaxError:
            pass
    text = ast.unparse(mod) + "\n"
    return _indent(text, " " * indent) if member and indent else text

def equivalent(a, b, member):
    def d(s):
        try:
            return ast.dump(strip_docstrings(ast.parse(_wrapped(s, member))))
        except SyntaxError:
            return None
    da, db = d(a), d(b)
    return da is not None and da == db

def validate(source):
    try:
        ast.parse(source)
    except SyntaxError as e:
        raise VtokError(f"result is not valid Python: {e.msg}", e.lineno)
