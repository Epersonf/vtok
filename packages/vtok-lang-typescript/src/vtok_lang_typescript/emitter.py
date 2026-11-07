"""Keyword style (`style != "native"`): statements become indented lines whose leading keyword is one character."""
from .emit import flat, join
from .grammar import KW, MODS, BARE_OK
from .nodes import SKIP, text

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
