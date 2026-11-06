"""Native-tight profile: real Python, minimal whitespace.

Same tokens as ordinary Python, but 1-space indentation, no optional spaces, single-statement
bodies inlined after the header and simple statements joined with ';'. The decoder is just
`ast.parse` + alias reversal + `ast.unparse`, so the grammar prompt is tiny and a model already
knows the notation."""
import ast, re
from .lexer import lex, wordlike
from vtok.errors import VtokError

_COMPOUND = re.compile(r"(def|class|if|elif|else|for|while|try|except|finally|with|async|match|case)\b|@")
_NOSEMI = re.compile(r"(import|from)\b")

def _tight(text):
    out, prev = [], None
    for t in lex(text):
        if prev is not None and ((wordlike(prev) and wordlike(t)) or (prev[0] == "NUM" and prev[1].isdigit() and t == ("OP", "."))):
            out.append(" ")
        out.append(t[1]); prev = t
    return "".join(out)

def render(py_text):
    parsed = []
    for ln in py_text.splitlines():
        if not ln.strip(): continue
        s = ln.lstrip(" ")
        parsed.append([(len(ln) - len(s)) // 4, _tight(s)])
    # inline a header with a single simple-statement body
    out, i = [], 0
    while i < len(parsed):
        ind, txt = parsed[i]
        if txt.endswith(":") and i + 1 < len(parsed):
            bi, bt = parsed[i + 1]
            nxt = parsed[i + 2][0] if i + 2 < len(parsed) else 0
            if bi == ind + 1 and nxt <= ind and not bt.endswith(":") and not _COMPOUND.match(bt) \
               and not txt.startswith(("class ", "def ", "async def ", "@")):
                out.append([ind, txt + bt]); i += 2; continue
        out.append([ind, txt]); i += 1
    # join consecutive simple statements of the same level
    res = []
    for ind, txt in out:
        if res and res[-1][0] == ind and _simple(res[-1][1]) and _simple(txt):
            res[-1][1] += ";" + txt
        else:
            res.append([ind, txt])
    return "\n".join(" " * i + t for i, t in res) + "\n"

def _simple(t):
    """A line that is a plain simple statement (no header, no inlined compound body)."""
    return not t.endswith(":") and not _COMPOUND.match(t) and not _has_inline_header(t)

def _has_inline_header(t):
    toks = lex(t)
    if not toks or toks[0][0] != "NAME": return False
    if toks[0][1] in ("if", "elif", "else", "for", "while", "try", "except", "finally", "with", "def", "class", "async"):
        return True
    return False

def parse(vt):
    try:
        return ast.parse(vt)
    except SyntaxError as e:
        raise VtokError(e.msg, e.lineno)
