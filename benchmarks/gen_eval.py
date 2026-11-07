"""Evaluate vtok's generic tree-sitter plugins (css html sql java cpp cs go c dart) on real source trees.

For every language: how many files encode (roundtrip-verified) and how many are refused, the mean token saving of
each view (full, skeleton, lean) over the files that encode, the median time of one verified encode, and three
sample `vtok outline` lines.

Corpus roots come from arguments or environment variables (nothing machine-specific is built in):
    python benchmarks/gen_eval.py css=~/src/site sql=~/src/app/migrations dart=~/flutter/packages
    VTOK_EVAL_GO=~/go/src VTOK_EVAL_C=~/src/linux:~/src/sqlite python benchmarks/gen_eval.py
    python benchmarks/gen_eval.py c=@files.txt          # @FILE: one path per line instead of a root
Options: --vtok PATH (default $VTOK_BIN or `vtok` on PATH), --max N files per language (default 300, evenly
sampled from the sorted list), --jobs N (default 8), --fails (print every refused file with the reason).
"""
import os
import re
import statistics
import subprocess
import sys
import time
from concurrent.futures import ThreadPoolExecutor

EXTS = {
    "css": [".css"], "html": [".html", ".htm"], "sql": [".sql"], "java": [".java"], "go": [".go"],
    "cpp": [".cpp", ".cc", ".cxx", ".hpp", ".hh", ".hxx"], "cs": [".cs"], "c": [".c", ".h"], "dart": [".dart"],
}
VIEWS = ["full", "skeleton", "lean"]
SKIP_DIRS = {".git", "node_modules", "target", "build", ".dart_tool", "bin", "obj", "vendor"}


def msys(p):
    """Git Bash style `/c/x` -> `c:/x` on Windows, so lists written from a bash `find` work as they are."""
    m = re.match(r"^/([a-zA-Z])/(.*)", p) if os.name == "nt" else None
    return f"{m.group(1)}:/{m.group(2)}" if m else p


def corpus(lang, spec):
    """Files of `lang` under the roots in `spec` (os.pathsep-separated; `@FILE` = list of paths)."""
    files = []
    for root in filter(None, spec.split(os.pathsep) if not spec.startswith("@") else [spec]):
        if root.startswith("@"):
            with open(os.path.expanduser(root[1:]), encoding="utf-8") as fh:
                files += [msys(ln.strip()) for ln in fh if ln.strip()]
            continue
        root = msys(root)
        for d, dirs, names in os.walk(os.path.expanduser(root)):
            dirs[:] = [x for x in dirs if x not in SKIP_DIRS]
            files += [os.path.join(d, n) for n in names if any(n.endswith(e) for e in EXTS[lang])]
    return sorted(set(files))


def sample(files, n):
    if len(files) <= n:
        return files
    step = len(files) / n
    return [files[int(i * step)] for i in range(n)]


def run(vtok, args):
    t = time.perf_counter()
    p = subprocess.run([vtok, *args], capture_output=True, text=True, encoding="utf-8", errors="replace")
    return p.stdout, p.stderr, time.perf_counter() - t


def measure(vtok, path):
    """{view: saving or None}, seconds of the verified full encode, the refusal reason (or "")."""
    res, secs, why = {}, 0.0, ""
    for v in VIEWS:
        out, err, dt = run(vtok, ["stats", path, "--view", v])
        if v == "full":
            secs = dt
        m = re.search(r"raw=(\d+) vtok\+table=(\d+)", out)
        ok = "ok=1" in out and m and int(m.group(1)) > 0
        res[v] = 1 - int(m.group(2)) / int(m.group(1)) if ok else None
        if v == "full" and not ok:
            why = (err.strip().splitlines() or ["?"])[-1]
            break
    return res, secs, why


def outline_lines(vtok, paths, k=3):
    for p in paths:
        out, _, _ = run(vtok, ["outline", p])
        lines = [ln for ln in out.splitlines() if ln.strip() and not ln.startswith("imports")]
        if len(lines) >= k:
            return lines[:k]
    return []


def evaluate(vtok, lang, files, jobs, show_fails):
    with ThreadPoolExecutor(jobs) as ex:
        rows = list(ex.map(lambda f: (f, *measure(vtok, f)), files))
    good = [r for r in rows if r[1].get("full") is not None]
    bad = [r for r in rows if r[1].get("full") is None]
    mean = lambda v: statistics.mean(r[1][v] for r in good if r[1].get(v) is not None) if good else 0.0
    print(f"\n== {lang}: files={len(rows)} ok={len(good)} refused={len(bad)} ({100 * len(bad) / max(1, len(rows)):.1f}%)")
    print("   mean saving: " + "  ".join(f"{v}={100 * mean(v):.1f}%" for v in VIEWS)
          + f"   median encode {1000 * statistics.median(r[2] for r in rows):.0f} ms" if rows else "")
    for ln in outline_lines(vtok, [r[0] for r in good]):
        print("   outline: " + ln[:100])
    for f, _, _, why in bad if show_fails else bad[:3]:
        print(f"   refused: {f}: {why[:160]}")
    return len(rows), len(good)


def main(argv):
    vtok = os.environ.get("VTOK_BIN", "vtok")
    nmax, jobs, show_fails, specs = 300, 8, False, {}
    it = iter(argv)
    for a in it:
        if a == "--vtok":
            vtok = next(it)
        elif a == "--max":
            nmax = int(next(it))
        elif a == "--jobs":
            jobs = int(next(it))
        elif a == "--fails":
            show_fails = True
        elif "=" in a and a.split("=", 1)[0] in EXTS:
            k, v = a.split("=", 1)
            specs[k] = v
        else:
            sys.exit(__doc__)
    for lang in EXTS:
        specs.setdefault(lang, os.environ.get(f"VTOK_EVAL_{lang.upper()}", ""))
    missing = [lang for lang, spec in specs.items() if not spec]
    for lang, spec in specs.items():
        files = sample(corpus(lang, spec), nmax) if spec else []
        if files:
            evaluate(vtok, lang, files, jobs, show_fails)
    if missing:
        print(f"\nno corpus for: {' '.join(missing)} (set VTOK_EVAL_<LANG> or pass lang=ROOT)")


if __name__ == "__main__":
    main(sys.argv[1:])
