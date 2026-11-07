"""Cross-file questions: where is NAME defined and who uses it?  Compare `vtok refs` with `grep -rnw` (what an agent reads
without it). Deterministic, no model. Tokens: tiktoken o200k_base (proxy).

Recall check: every grep hit that is code (not a comment line) should appear in the refs output, as a use line or a
definition start. Hits inside strings count as misses (a name in a string is not a reference), reported as such."""
import os
os.environ.setdefault("VTOK_NO_USAGE", "1")  # benchmark runs must not enter the user's usage log
import argparse, json, pathlib, re, subprocess, sys
import tiktoken

ENC = tiktoken.get_encoding("o200k_base")
REPO = pathlib.Path(__file__).resolve().parents[2]
ntok = lambda s: len(ENC.encode(s))

TARGETS = [
    (REPO / "rust", "*.rs", ["compact", "summarize", "symbols", "outline", "language_for_path", "ntok", "score", "store", "resolve_chunks", "apply_edits", "Options", "tokens"]),
    (REPO / ".venv/lib/python3.12/site-packages/requests", "*.py", ["Session", "prepare_request", "merge_setting", "HTTPAdapter", "PreparedRequest", "urlparse", "get_adapter", "RequestException"]),
]


def grep(name, root, glob):
    r = subprocess.run(["grep", "-rnw", f"--include={glob}", "--exclude-dir=target", name, str(root)], capture_output=True, text=True)
    return r.stdout


def py_truth(name, root):
    """(file, line) of every NAME token that Python's own tokenizer sees (so comments and strings are excluded)."""
    import io, tokenize
    hits = set()
    for f in pathlib.Path(root).rglob("*.py"):
        try: src = f.read_text()
        except Exception: continue
        if name not in src: continue
        try:
            for t in tokenize.generate_tokens(io.StringIO(src).readline):
                if t.type == tokenize.NAME and t.string == name: hits.add((str(f.relative_to(root)), t.start[0]))
        except Exception: pass
    return hits


def parse_refs(out):
    hits = set()
    for l in out.splitlines():
        if l.startswith("def "):
            m = re.match(r"def (\S+) .* L(\d+)(?:-\d+)?(?: \(.*\))?$", l)
            if m: hits.add((m.group(1), int(m.group(2))))
        elif l.startswith("use "):
            m = re.match(r"use (\S+) .* @ (L[\d,L]+)$", l)
            if m:
                for n in m.group(2).replace("L", "").split(","): hits.add((m.group(1), int(n)))
    return hits


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("out")
    ap.add_argument("--vtok", default=str(REPO / "rust/target/release/vtok"))
    a = ap.parse_args()
    rows = []
    for root, glob, names in TARGETS:
        for name in names:
            g = grep(name, root, glob)
            g_lines = [l for l in g.splitlines() if l]
            r = subprocess.run([a.vtok, "refs", name, str(root), "--max-tokens", "800"], capture_output=True, text=True).stdout
            hits = parse_refs(r)
            code, missing_code, miss_other = 0, [], 0
            if glob == "*.py":
                truth = py_truth(name, root)
                code = len(truth)
                missing_code = sorted(truth - hits)
                g_lines = g_lines  # tokens above still report the grep cost
                g_lines_for_check = []
            else:
                g_lines_for_check = g_lines
            for l in g_lines_for_check:
                m = re.match(r"(.+?):(\d+):(.*)$", l)
                if not m: continue
                rel = str(pathlib.Path(m.group(1)).relative_to(root)); ln = int(m.group(2)); text = m.group(3).strip()
                if text.startswith(("//", "#", "*", "/*", '"""')): continue
                code += 1
                if (rel, ln) not in hits:
                    # a name inside a string / doc line is not a reference
                    inside_str = bool(re.search(r"[\"'][^\"']*\b%s\b[^\"']*[\"']" % re.escape(name), text))
                    (miss_other if inside_str else missing_code.append((rel, ln, text[:70]))) if False else None
                    if inside_str: miss_other += 1
                    else: missing_code.append((rel, ln, text[:70]))
            rows.append({"name": name, "root": root.name, "grep_tokens": ntok(g), "grep_hits": len(g_lines), "refs_tokens": ntok(r), "code_hits": code,
                         "missed_code_hits": len(missing_code), "hits_in_strings": miss_other, "missed_examples": [list(m)[:3] for m in missing_code[:3]], "truncated": "more use sites" in r})
    pathlib.Path(a.out).write_text(json.dumps(rows, indent=1))
    print("%-20s %-9s %8s %8s %6s %9s %7s" % ("name", "tree", "grep tok", "refs tok", "ratio", "code hits", "missed"))
    for x in rows:
        print("%-20s %-9s %8d %8d %6.2f %9d %7d%s" % (x["name"], x["root"], x["grep_tokens"], x["refs_tokens"], x["refs_tokens"] / max(1, x["grep_tokens"]), x["code_hits"], x["missed_code_hits"], " T" if x["truncated"] else ""))
    big = sorted(x["refs_tokens"] / max(1, x["grep_tokens"]) for x in rows if x["grep_tokens"] > 150)
    print("median refs/grep over %d names with grep > 150 tokens: %.2f; names where refs is larger: %d of %d" % (len(big), big[len(big) // 2] if big else 0, sum(1 for x in rows if x["refs_tokens"] > x["grep_tokens"]), len(rows)))
    print("total code hits %d, missed %d (hits inside strings: %d)" % (sum(x["code_hits"] for x in rows), sum(x["missed_code_hits"] for x in rows), sum(x["hits_in_strings"] for x in rows)))


if __name__ == "__main__":
    main()
