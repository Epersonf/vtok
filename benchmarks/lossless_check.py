"""Independent losslessness check for vtok encode/decode (not the plugin's own proof).

usage: python lossless_check.py LANG FILELIST [MAX]
For each file: vtok encode (with table + literals), vtok decode, then compare original and decoded after a separate
tokenisation: comments removed, string literals kept exactly, words kept whole, whitespace dropped except where it
separates two things that would merge (word|word, and for css also word|selector-start). A mismatch is reported with the
first differing token.
"""
import os
import re
import subprocess
import sys
import tempfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from lossless_lex import norm  # noqa: E402

VT = os.environ.get("VTOK_BIN", os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "rust", "target", "release", "vtok.exe" if os.name == "nt" else "vtok"))
LANGS = {"dart": "dart", "c": "c", "cpp": "cpp", "cs": "csharp", "sql": "sql", "css": "css", "html": "html", "java": "java", "go": "go"}


def run(a, **kw):
    r = subprocess.run(a, capture_output=True, **kw)
    r.stdout = r.stdout.decode("utf-8", errors="replace")
    r.stderr = r.stderr.decode("utf-8", errors="replace")
    return r


def check(path, lang):
    code = LANGS[lang]
    with tempfile.TemporaryDirectory() as d:
        tbl, lit = os.path.join(d, "t"), os.path.join(d, "l")
        e = run([VT, "encode", path, "--lang", code, "--view", "full", "--table", tbl, "--literals", lit])
        if e.returncode != 0:
            return "refused"
        cf = os.path.join(d, "c")
        open(cf, "w", encoding="utf-8", newline="").write(e.stdout)  # bytes preserved: run() decodes without newline translation
        extra = (["--table", tbl] if os.path.exists(tbl) else []) + (["--literals", lit] if os.path.exists(lit) else [])
        dec = run([VT, "decode", cf, "--lang", code] + extra)
        if dec.returncode != 0:
            return "decode-error: " + dec.stderr.strip()[:120]
        a = norm(open(path, encoding="utf-8", errors="replace", newline="").read(), lang)
        b = norm(dec.stdout, lang)
        if a == b:
            return "ok"
        for k in range(min(len(a), len(b))):
            if a[k] != b[k]:
                return f"MISMATCH at token {k}: {a[max(0,k-2):k+3]} vs {b[max(0,k-2):k+3]}"
        return f"MISMATCH length {len(a)} vs {len(b)}"


def main():
    lang, lst = sys.argv[1], sys.argv[2]
    mx = int(sys.argv[3]) if len(sys.argv) > 3 else 10**9
    files = [re.sub(r"^/([a-zA-Z])/", r"\1:/", l.strip()) for l in open(lst, encoding="utf-8") if l.strip()][:mx]
    res = {}
    bad = []
    for f in files:
        r = check(f, lang)
        key = r.split(":")[0].split(" at")[0]
        res[key] = res.get(key, 0) + 1
        if r != "ok" and r != "refused":
            bad.append((f, r))
    print(lang, len(files), res)
    for f, r in bad[:8]:
        print("  ", os.path.basename(f), "->", r[:300])


main()
