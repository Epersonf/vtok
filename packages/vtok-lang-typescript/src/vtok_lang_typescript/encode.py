"""TypeScript source -> Vtok-TS (tree-sitter CST walk)."""
from vtok.errors import VtokError
from vtok.aliasing import plan_aliases
from .grammar import KW, MODS, BARE_OK, RESERVED, KW_INV
from .parse import parse, first_error
from .canon import canon

IDENT = {"identifier", "type_identifier", "property_identifier"}
LIKE_IDENT = IDENT | {"property_identifier", "shorthand_property_identifier",
                      "shorthand_property_identifier_pattern", "statement_identifier",
                      "private_property_identifier"}
SKIP = {"comment", "empty_statement"}
JS_WORDS = {"abstract", "any", "as", "async", "await", "boolean", "break", "case", "catch", "class", "const",
            "continue", "debugger", "declare", "default", "delete", "do", "else", "enum", "export", "extends",
            "false", "finally", "for", "from", "function", "get", "if", "implements", "import", "in",
            "instanceof", "interface", "is", "keyof", "let", "module", "namespace", "never", "new", "null",
            "number", "object", "of", "override", "package", "private", "protected", "public", "readonly",
            "return", "satisfies", "set", "static", "string", "super", "switch", "symbol", "this", "throw",
            "true", "try", "type", "typeof", "undefined", "unknown", "var", "void", "while", "with", "yield"}

class Ctx:
    def __init__(self, ren, keywords, skeleton=False, lean=False, lits=None):
        self.ren, self.kw, self.skeleton, self.lean, self.lits = ren, keywords, skeleton, lean, lits or {}

def text(n): return n.text.decode()

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

def line_from(tokens, ctx, mapkw=True):
    """Join header tokens, mapping the leading keyword (after modifiers) to its 1-char form."""
    if not (ctx.kw and mapkw):
        return join(tokens)
    i, pre = 0, []
    while i < len(tokens) and tokens[i] in MODS:
        pre.append("X" if tokens[i] == "export" else tokens[i]); i += 1
    if i < len(tokens) and tokens[i] in KW and tokens[i] != "export" and (i + 1 < len(tokens) or KW[tokens[i]] in BARE_OK):
        pre.append(KW[tokens[i]]); i += 1
        rest = join(tokens[i:])
        return " ".join(pre) + (" " + rest if rest else "")
    return " ".join(pre + [join(tokens[i:])]) if tokens[i:] else " ".join(pre)

def is_empty(body):
    return not any(c.type not in SKIP for c in body.named_children)

class Emitter:
    def __init__(self, ctx): self.ctx, self.lines = ctx, []
    def add(self, ind, s): self.lines.append(" " * ind + s)

    def block(self, body, ind):
        if body is None: return
        if body.type in ("statement_block", "class_body", "interface_body", "enum_body", "switch_body"):
            for c in body.named_children: self.stmt(c, ind)
        else:
            self.stmt(body, ind)

    def header(self, ind, tokens, body, mapkw=True):
        empty = body is not None and body.type in ("statement_block", "class_body", "interface_body", "enum_body", "switch_body") and is_empty(body)
        if self.ctx.skeleton and body is not None and body.type == "statement_block": empty = True
        self.add(ind, line_from(tokens, self.ctx, mapkw) + (" {}" if empty else ""))
        if not empty: self.block(body, ind + 1)

    def cond_line(self, ind, kwv, tsword, cond, body):
        inner = [x for ch in cond.children if ch.type not in ("(", ")") for x in flat(ch, self.ctx)]
        empty = body is not None and body.type == "statement_block" and is_empty(body)
        if self.ctx.kw:
            c = join(inner)
            s = "?" + c if kwv == "?" else f"{kwv} {c}"
        else:
            s = join(tsword.split() + ["("] + inner + [")"])
        self.add(ind, s + (" {}" if empty else ""))
        if not empty: self.block(body, ind + 1)

    def stmt(self, n, ind, prefix=()):
        t, ctx = n.type, self.ctx
        if t in SKIP: return
        before = lambda body: [x for c in n.children if c.start_byte < body.start_byte and c.type != "comment" for x in flat(c, ctx)]
        if t == "export_statement":
            d = n.child_by_field_name("declaration")
            if d is not None and d.type in ("function_declaration", "generator_function_declaration", "class_declaration",
                                           "abstract_class_declaration", "interface_declaration", "enum_declaration", "internal_module"):
                pre = [x for c in n.children if c.start_byte < d.start_byte and c.type != "comment" for x in flat(c, ctx)]
                return self.stmt(d, ind, tuple(prefix) + tuple(pre))
        if t in ("function_declaration", "generator_function_declaration", "method_definition", "class_declaration",
                 "abstract_class_declaration", "interface_declaration", "enum_declaration", "internal_module"):
            body = n.child_by_field_name("body")
            if body is not None:
                return self.header(ind, list(prefix) + before(body), body)
        if t == "if_statement": return self.if_stmt(n, ind, False)
        if t == "while_statement":
            return self.cond_line(ind, "w", "while", n.child_by_field_name("condition"), n.child_by_field_name("body"))
        if t == "switch_statement":
            body = n.child_by_field_name("body")
            self.cond_line(ind, "S", "switch", n.child_by_field_name("value"), None)
            if body is not None and is_empty(body): self.lines[-1] += " {}"; return
            return self.block(body, ind + 1)
        if t in ("for_statement", "for_in_statement"):
            body = n.child_by_field_name("body")
            toks = before(body)
            if ctx.kw and toks and toks[0] == "for":
                j = 2 if toks[1:2] == ["await"] else 1
                if toks[j:j + 1] == ["("] and toks[-1:] == [")"]:
                    inner = toks[j + 1:-1]
                    if len(inner) > 1 and inner[0] in ("const", "let", "var"):
                        inner[0] = {"const": "k ", "let": "l ", "var": "v "}[inner[0]]
                    toks = toks[:j] + inner
            return self.header(ind, toks, body)
        if t in ("switch_case", "switch_default"):
            val = n.child_by_field_name("value")
            head = ["case"] + flat(val, ctx) if t == "switch_case" else ["default"]
            self.add(ind, join(head) + ":")
            for c in n.named_children:
                if val is not None and c == val: continue
                self.stmt(c, ind + 1)
            return
        if t == "try_statement":
            self.header(ind, ["try"], n.child_by_field_name("body"))
            h, f = n.child_by_field_name("handler"), n.child_by_field_name("finalizer")
            if h is not None:
                hb = h.child_by_field_name("body")
                self.header(ind, [x for c in h.children if c.start_byte < hb.start_byte and c.type != "comment" for x in flat(c, ctx)], hb)
            if f is not None:
                self.header(ind, ["finally"], f.child_by_field_name("body"))
            return
        if t == "hash_bang_line": return self.add(ind, text(n))
        toks = list(prefix) + flat(n, ctx)
        while toks and toks[-1] == ";": toks.pop()
        self.add(ind, line_from(toks, ctx))

    def if_stmt(self, n, ind, is_elif):
        ctx = self.ctx
        cons = n.child_by_field_name("consequence")
        if ctx.kw:
            self.cond_line(ind, "e" if is_elif else "?", "", n.child_by_field_name("condition"), cons)
        else:
            self.cond_line(ind, "", "else if" if is_elif else "if", n.child_by_field_name("condition"), cons)
        alt = n.child_by_field_name("alternative")
        if alt is None: return
        kids = [c for c in alt.named_children if c.type != "comment"]
        body = kids[0] if kids else None
        if body is not None and body.type == "if_statement":
            return self.if_stmt(body, ind, True)
        empty = body is None or (body.type == "statement_block" and is_empty(body))
        self.add(ind, ("E" if ctx.kw else "else") + (" {}" if empty else ""))
        if not empty: self.block(body, ind + 1)

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

def encode(source, alias=True, verify=True, keywords=True, table=None, view="full", style="native", hide=0, lits_out=None):
    source = source.replace("\r\n", "\n")
    root = parse(source.encode()).root_node
    err = first_error(root)
    if err is not None:
        raise VtokError(f"source has syntax errors near line {err.start_point[0] + 1}")
    if table is not None:
        table = dict(table)
    elif alias:
        counts, excl, avoid = _excluded_and_counts(root)
        bad = [n for n in counts if n in RESERVED and n in excl]
        if bad: raise VtokError(f"unsupported: imported/exported name collides with a Vtok token: {bad}")
        table = plan_aliases(counts, avoid=avoid, reserved=RESERVED, excluded=excl)
    else:
        table = {}
        counts, excl, _ = _excluded_and_counts(root)
        bad = [n for n in counts if n in RESERVED]
        if bad: table = plan_aliases({n: 1 for n in bad}, avoid=set(), reserved=RESERVED)
    lits = {}
    pos = {}
    if hide:
        pos, lits = collect_literals(root, hide)
        if lits_out is not None: lits_out.update(lits)
    if style == "native":
        vt = "\n".join(native_lines(root.named_children, Ctx(table, False, view == "skeleton", view == "lean", pos))) + "\n"
    else:
        em = Emitter(Ctx(table, keywords, view == "skeleton"))
        for c in root.named_children: em.stmt(c, 0)
        vt = "\n".join(em.lines) + "\n"
    if verify and view != "lean":   # lean output (import{names}) is intentionally not valid TS
        from .decode import decode
        back = decode(vt, {a: o for o, a in table.items()}, style=style, literals=lits)
        if view == "full" and canon(parse(back.encode()).root_node) != canon(root):
            raise VtokError("roundtrip verification failed (tree differs)")
    return vt, table
