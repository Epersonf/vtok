"""Native notation: a syntax node becomes tokens (`flat`), tokens are joined with the spaces they need, statements become lines."""
from vtok.errors import VtokError
from .nodes import IDENT, SKIP, text

class Ctx:
    def __init__(self, ren, keywords, skeleton=False, lean=False, lits=None):
        self.ren, self.kw, self.skeleton, self.lean, self.lits = ren, keywords, skeleton, lean, lits or {}

def _isw(c): return c.isalnum() or c in "_$"

def need_space(a, b):
    x, y = a[-1], b[0]
    if _isw(x) and _isw(y): return True
    if x == y and x in "+-": return True
    if x == "/" and y in "/*": return True
    if x == "?" and y == ".": return True
    if x == "!" and y == "=": return True
    if a.isdigit() and y == ".": return True
    return False

def join(tokens):
    out = []
    for i, t in enumerate(tokens):
        if i and need_space(tokens[i - 1], t): out.append(" ")
        out.append(t)
    return "".join(out)

def flat(n, ctx):
    t = n.type
    if t == "comment": return []
    if t == "string" and n.start_byte in ctx.lits: return [ctx.lits[n.start_byte]]
    if ctx.lean:
        if t == "type_annotation": return []
        if t == "import_statement":
            ids, st = [], [n]
            while st:
                x = st.pop(); st.extend(reversed(x.children))
                if x.type == "identifier": ids.append(text(x))
            return ["import{" + ",".join(ids) + "}", ";"] if ids else []
    if t in IDENT:
        s = text(n); return [ctx.ren.get(s, s)]
    if t == "template_string": return [template(n, ctx)]
    if ctx.skeleton and t in ("arrow_function", "function_expression", "function", "generator_function", "function_declaration", "generator_function_declaration", "method_definition"):
        b = n.child_by_field_name("body")
        if b is not None and b.type == "statement_block":
            return [x for c in n.children if c.start_byte < b.start_byte and c.type != "comment" for x in flat(c, ctx)] + ["{}"]
    if t == "template_literal_type":
        parts = []
        for c in n.children:
            if c.type == "template_type":
                parts.append("${" + join([x for ch in c.children if ch.type not in ("${", "}") for x in flat(ch, ctx)]) + "}")
            else:
                parts.append(text(c))
        return ["".join(parts)]
    if n.child_count == 0 or t in ("string", "regex"):
        s = text(n)
        if "\n" in s: raise VtokError(f"unsupported: newline inside {t}")
        return [s]
    out = []
    for c in n.children: out += flat(c, ctx)
    return out

def template(n, ctx):
    tagged = n.parent is not None and n.parent.type == "call_expression"
    parts = []
    for c in n.children:
        if c.type == "template_substitution":
            inner = [x for ch in c.children if ch.type not in ("${", "}") for x in flat(ch, ctx)]
            parts.append("${" + join(inner) + "}")
        else:
            s = text(c)
            if "\n" in s:
                if tagged: raise VtokError("unsupported: multiline tagged template")
                s = s.replace("\n", "\\n")
            parts.append(s)
    return "".join(parts)

def flat_stmt(n, ctx):
    toks = flat(n, ctx)
    return [t for i, t in enumerate(toks) if not (t == ";" and i + 1 < len(toks) and toks[i + 1] == "}")]

def native_lines(nodes, ctx):
    """One statement per line, minified. A trailing ';' is dropped only where the next line cannot
    continue the expression (ASI-safe)."""
    lines = [join(flat_stmt(c, ctx)) for c in nodes if c.type not in SKIP]
    for i, l in enumerate(lines):
        nxt = lines[i + 1] if i + 1 < len(lines) else ""
        if l.endswith(";") and (not nxt or nxt[0].isalpha() or nxt[0] in "_$@"):
            lines[i] = l[:-1]
    return lines
