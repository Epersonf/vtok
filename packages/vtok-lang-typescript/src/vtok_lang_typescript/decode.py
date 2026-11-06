"""Vtok-TS -> TypeScript. Line-based; validated with tree-sitter (errors carry Vtok line numbers)."""
import re
from vtok.errors import VtokError
from .grammar import KW_INV, MODS, BARE_OK, COND, BLOCKISH
from .parse import parse, first_error

WORD = re.compile(r"[A-Za-z_$][\w$]*")
IDENT = {"identifier", "type_identifier", "property_identifier"}

class Node:
    __slots__ = ("ind", "text", "line", "kids")
    def __init__(self, ind, text, line): self.ind, self.text, self.line, self.kids = ind, text, line, []

def parse_lines(vtok):
    root, stack = Node(-1, "", 0), []
    stack = [root]
    for n, raw in enumerate(vtok.splitlines(), 1):
        if not raw.strip(): continue
        s = raw.lstrip(" ")
        if s.startswith("\t") or raw.startswith("\t"):
            raise VtokError("tabs are not allowed; indent with 1 space per level", n)
        ind = len(raw) - len(s)
        while stack[-1].ind >= ind: stack.pop()
        if ind > stack[-1].ind + 1:
            raise VtokError(f"unexpected indent (depth {ind}, expected at most {stack[-1].ind + 1})", n)
        node = Node(ind, s.rstrip(), n)
        stack[-1].kids.append(node); stack.append(node)
    return root

def head_word(ts):
    s, words = ts, []
    while True:
        s = s.lstrip(" ")
        m = WORD.match(s)
        if not m: break
        w = m.group(0); s = s[m.end():]
        if w in MODS and w != "default" or (w == "default" and not s.startswith(":")):
            continue
        words.append(w)
        if len(words) == 2 or w != "const": break
    if not words and ts.startswith("default:"): return "default"
    if words and words[0] == "const" and len(words) > 1 and words[1] == "enum": return "enum"
    return words[0] if words else ""

def cond(tsword, c):
    marker = c.endswith(" {}")
    if marker: c = c[:-3]
    return f"{tsword} ({c.strip()})" + (" {}" if marker else "")

def _whole_parens(s):
    if not (s.startswith("(") and s.endswith(")")):
        return False
    depth = 0
    for i, c in enumerate(s):
        depth += (c == "(") - (c == ")")
        if depth == 0 and i < len(s) - 1:
            return False
    return depth == 0

def expand_for(rest):
    """`k x of xs` -> `(const x of xs)`; explicit `(...)` is kept; `await` prefix supported."""
    rest = rest.strip()
    marker = rest.endswith(" {}")
    if marker: rest = rest[:-3]
    pre = ""
    if rest.startswith("await "):
        pre, rest = "await ", rest[6:].lstrip()
    if not _whole_parens(rest):
        m = re.match(r"([klv]) (?=[\[{A-Za-z_$])", rest)
        if m and not re.match(r"(of|in)\b", rest[m.end():]):
            rest = {"k": "const ", "l": "let ", "v": "var "}[m.group(1)] + rest[m.end():]
        rest = "(" + rest + ")"
    return pre + rest + (" {}" if marker else "")

def map_line(text, bare=True):
    s, acc = text, ""
    if s.startswith("?"):
        return cond("if", s[1:])
    while True:
        lead = len(s) - len(s.lstrip(" ")); acc += s[:lead]; s = s[lead:]
        m = WORD.match(s)
        if not m: break
        w, after = m.group(0), s[m.end():]
        if w in KW_INV and (after.startswith(" ") or (after == "" and w in BARE_OK and bare)):
            if w in COND:
                return acc + cond(KW_INV[w], after)
            if w == "L":
                return acc + "for " + expand_for(after)
            acc += KW_INV[w]; s = after
            if KW_INV[w] == "export": continue
            break
        if w in MODS:
            acc += w; s = after; continue
        break
    return acc + s

class Out:
    def __init__(self): self.lines, self.src = [], []
    def add(self, s, ln): self.lines.append(s); self.src.append(ln)

def render(nodes, parent, out, depth=0):
    for n in nodes:
        pad = "  " * depth
        ts = map_line(n.text, parent not in ("enum", "class", "interface"))
        head = head_word(ts)
        if head in ("case", "default") and (head == "case" or ts.startswith("default:")):
            out.add(pad + ts, n.line); render(n.kids, "case", out, depth + 1); continue
        if n.kids:
            out.add(pad + ts + " {", n.line); render(n.kids, head, out, depth + 1); out.add(pad + "}", n.line)
        else:
            term = ";"
            if parent == "enum": term = ","
            if head in BLOCKISH or ts.startswith(("@", "#!")) or ts.endswith(" {}") and head in BLOCKISH: term = ""
            out.add(pad + ts + term, n.line)

def decode(vtok, table=None, style="native", literals=None):
    if style == "native":
        code = vtok if vtok.endswith("\n") else vtok + "\n"
        return _finish(code, table, lambda row: row + 1, literals)
    out = Out()
    render(parse_lines(vtok).kids, "", out)
    code = "\n".join(out.lines) + "\n"
    return _finish(code, table, lambda row: out.src[min(row, len(out.src) - 1)])

def _finish(code, table, srcline, literals=None):
    raw = code.encode()
    root = parse(raw).root_node
    err = first_error(root)
    if err is not None:
        row = min(err.start_point[0], len(code.splitlines()) - 1)
        raise VtokError(f"syntax error near {code.splitlines()[row].strip()[:60]!r}", srcline(row))
    if table or literals:
        spans, stack = [], [root]
        while stack:
            n = stack.pop(); stack.extend(n.children)
            if literals and n.type == "string" and n.text.decode() in literals:
                spans.append((n.start_byte, n.end_byte, literals[n.text.decode()]))
            elif table and n.type in IDENT and n.text.decode() in table:
                spans.append((n.start_byte, n.end_byte, table[n.text.decode()]))
        for a, b, r in sorted(spans, reverse=True):
            raw = raw[:a] + r.encode() + raw[b:]
        code = raw.decode()
    return code
