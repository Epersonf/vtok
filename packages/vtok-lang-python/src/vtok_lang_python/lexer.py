"""Tiny shared lexer for python-ish text (used by encoder line transform and decoder)."""
import re

from vtok.errors import VtokError

STR = (r"""(?:[rRbBuUfF]{1,2})?(?:'''(?:\\.|[^\\])*?'''|\"\"\"(?:\\.|[^\\])*?\"\"\"|'(?:\\.|[^'\\\n])*'|"(?:\\.|[^"\\\n])*")""")
NUM = r"(?:0[xXoObB][0-9a-fA-F_]+|(?:\d[\d_]*\.?[\d_]*|\.\d[\d_]*)(?:[eE][+-]?\d+)?[jJ]?)"
NAME = r"[^\W\d]\w*"
OPS = r"\|\|=|&&=|\|\||&&|\*\*=|//=|>>=|<<=|\.\.\.|->|:=|==|!=|<=|>=|\*\*|//|>>|<<|\+=|-=|\*=|/=|%=|@=|&=|\|=|\^=|[-+*/%@&|^~<>=()\[\]{},:.;!?]"
TOKEN = re.compile(rf"\s*(?:(?P<STR>{STR})|(?P<NUM>{NUM})|(?P<NAME>{NAME})|(?P<OP>{OPS}))")

_START = re.compile(r"\s*([rRbBuUfF]{1,2})?('''|\"\"\"|'|\")")

def _scan_str(t, i, q, fmt, raw):
    """i = index after opening quote; returns index after closing quote. f-string aware (PEP 701)."""
    depth, n = 0, len(t)
    while i < n:
        c = t[i]
        if c == "\\" and not raw:
            i += 2; continue
        if depth == 0 and t.startswith(q, i):
            return i + len(q)
        if fmt:
            if c == "{":
                if depth == 0 and t.startswith("{{", i): i += 2; continue
                depth += 1
            elif c == "}":
                if depth == 0 and t.startswith("}}", i): i += 2; continue
                depth = max(depth - 1, 0)
            elif depth > 0 and c in "\"'":
                pre = t[max(i - 2, 0):i]
                pre = pre[-2:] if all(ch in "rRbBuUfF" for ch in pre[-2:]) else (pre[-1:] if pre[-1:] in list("rRbBuUfF") else "")
                nq = c * 3 if t.startswith(c * 3, i) else c
                i = _scan_str(t, i + len(nq), nq, "f" in pre.lower(), "r" in pre.lower())
                continue
        i += 1
    raise VtokError("unterminated string")

def lex(text, line=None):
    """Return list of (kind, value). Raises VtokError on unlexable input."""
    out, pos, n = [], 0, len(text)
    while pos < n:
        if text[pos:].strip() == "":
            break
        ms = _START.match(text, pos)
        if ms and ms.group(1) and "f" in ms.group(1).lower():
            try:
                end = _scan_str(text, ms.end(), ms.group(2), True, "r" in ms.group(1).lower())
            except VtokError as e:
                raise VtokError(str(e), line)
            out.append(("STR", text[ms.start() + (len(text[ms.start():]) - len(text[ms.start():].lstrip())):end]))
            pos = end
            continue
        m = TOKEN.match(text, pos)
        if not m:
            raise VtokError(f"cannot tokenize near {text[pos:pos+12]!r}", line)
        kind = m.lastgroup
        out.append((kind, m.group(kind)))
        pos = m.end()
    return out

def wordlike(tok):
    return tok[0] in ("NAME", "NUM", "STR")
