"""Tokeniser for lossless_check.py: comments removed, strings kept exactly, words whole, spacing kept only where
it separates two things that would otherwise merge (word|word; for css, in selectors and @media preludes)."""
import re

WORD = re.compile(r"[A-Za-z0-9_$@#\u0080-\uffff]")


def tokens(text, lang):
    t, i, n, out = text.lstrip("\ufeff"), 0, len(text.lstrip("\ufeff")), []
    sql = lang == "sql"
    css = lang == "css"
    html = lang == "html"
    ws_before = False
    while i < n:
        c = t[i]
        if c.isspace():
            ws_before = True
            i += 1
            continue
        if html and t.startswith("<!--", i):
            j = t.find("-->", i + 4)
            i = n if j < 0 else j + 3
            continue
        if not html and t.startswith("/*", i):
            j = t.find("*/", i + 2)
            i = n if j < 0 else j + 2
            continue
        if (t.startswith("//", i) and not css and not html and not sql) or (sql and t.startswith("--", i)):
            j = t.find("\n", i)
            i = n if j < 0 else j
            continue
        if sql and c == "$":
            m = re.match(r"\$[A-Za-z_0-9]*\$", t[i:])
            if m:
                j = t.find(m.group(0), i + len(m.group(0)))
                j = n if j < 0 else j + len(m.group(0))
                out.append((ws_before, t[i:j]))
                ws_before, i = False, j
                continue
        if c in "\"'`" and not html:
            j = i + 1
            while j < n:
                if t[j] == "\\" and not sql and not (lang == "go" and c == "`"):
                    j += 2
                    continue
                if t[j] == c:
                    if sql and j + 1 < n and t[j + 1] == c:
                        j += 2
                        continue
                    break
                j += 1
            out.append((ws_before, t[i:j + 1]))
            ws_before, i = False, j + 1
            continue
        if WORD.match(c) or (css and c in "-.%"  and i + 1 < n and WORD.match(t[i + 1])):
            j = i + 1
            while j < n and (WORD.match(t[j]) or (css and t[j] in "-.%")):
                j += 1
                if css and t[j - 1] == "%":
                    break  # a percentage token ends at the percent sign
            out.append((ws_before, t[i:j]))
            ws_before, i = False, j
            continue
        out.append((ws_before, c))
        ws_before = False
        i += 1
    return out


def prelude_flags(toks):
    flags, state = [False] * len(toks), "end"
    for k in range(len(toks) - 1, -1, -1):
        tok = toks[k][1]
        if tok == "{":
            state = "open"
        elif tok in (";", "}"):
            state = "end"
        flags[k] = state == "open"
    return flags


def norm(text, lang):
    toks = tokens(text, lang)
    pre = prelude_flags(toks) if lang == "css" else None
    res = []
    for k, (ws, tok) in enumerate(toks):
        keep_ws = False
        if k and ws:
            prev = toks[k - 1][1]
            word_pair = WORD.match(prev[-1]) and WORD.match(tok[0])
            css_pair = lang == "css" and pre[k] and (prev[-1] in ")]*" or WORD.match(prev[-1])) and (tok[0] in ".#[:(&" or WORD.match(tok[0]))
            html_pair = lang == "html" and WORD.match(prev[-1]) and WORD.match(tok[0])
            keep_ws = bool(word_pair or css_pair or html_pair)
        res.append((" " if keep_ws else "") + tok)
    return res
