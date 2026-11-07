"""How much of a diff survives each compactor: raw git vs RTK vs vtok (old/new), on real commits.

usage: python diff_eval.py [--commits N] [--repo PATH]... [--rows]
Environment: RTK_BIN, VTOK_OLD, VTOK_NEW (executables; a missing one is skipped).

For every commit (the last N non-merge commits of each repo) two commands run, each tool in its native form:
  `git show <sha>` (commit header + patch) and `git diff <sha>~1 <sha>`;
  RTK as `rtk git show <sha>` / `rtk git diff A B`, vtok as `vtok run -- git ...`.
Plus the named row `git diff HEAD~3` of the first repo.

Metrics, per tool:
  tokens      (chars + 3) // 4 of everything the tool prints (RTK's own benchmark metric).
  keep        changed-line retention: of the added/removed lines of the raw patch (prefix `+`/`-`, not `+++`/`---`),
              take those whose stripped text has at least 4 alphanumeric characters (so `}` or `)` do not count);
              a line is retained when its stripped text occurs verbatim in the output, or, for lines of 60+ chars,
              when its first 60 characters do (a tool may cut long lines and say so).
  keep-src    the same, ignoring files that are lock/generated/vendored/minified/snapshots (see GENERATED).
  files       fraction of changed paths (the `b/` path, or `a/` for deletions) named somewhere in the output.
Rows are bucketed by raw size: small <= 400 tokens < medium <= 4000 < large. Per bucket: total tokens, median and
pooled retention (pooled = retained lines / changed lines over the whole bucket).
"""
import os
os.environ.setdefault("VTOK_NO_USAGE", "1")  # benchmark runs must not enter the user's usage log
import re
import shutil
import statistics
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
HOME = os.path.expanduser("~")
BIN = {
    "rtk": os.environ.get("RTK_BIN", r"C:\Users\epers\Documents\Repositories\OpenSource\rtk\target\release\rtk.exe"),
    "vtok-old": os.environ.get("VTOK_OLD", os.path.join(HOME, ".vtok", "bin", "vtok.exe")),
    "vtok-new": os.environ.get("VTOK_NEW", os.path.join(HERE, "..", "rust", "target", "release", "vtok.exe")),
}
BIN = {k: v for k, v in BIN.items() if v and (os.path.exists(v) or shutil.which(v))}
TOOLS = ["raw"] + list(BIN)
GENERATED = re.compile(r"(^|/)(Cargo\.lock|package-lock\.json|yarn\.lock|pnpm-lock\.yaml|go\.sum|poetry\.lock|uv\.lock|"
                       r"composer\.lock|Gemfile\.lock)$|\.min\.\w+$|(^|/)(dist|vendor|node_modules|__snapshots__)/|\.snap$")
env = dict(os.environ, NO_COLOR="1", GIT_PAGER="cat", PAGER="cat")


def tok(s):
    return (len(s) + 3) // 4


def sh(cmd, cwd):
    p = subprocess.run(cmd, cwd=cwd, capture_output=True, text=True, encoding="utf-8", errors="replace", env=env, timeout=300)
    return p.stdout + p.stderr


def changed(raw):
    """[(path, stripped line)] of the changed lines, and the list of changed paths."""
    out, paths, path = [], [], None
    for l in raw.split("\n"):
        if l.startswith("diff --git "):
            path = l.split(" b/")[-1].strip()
            paths.append(path)
        elif l.startswith("diff --cc ") or l.startswith("diff --combined "):
            path = l.split(" ", 2)[2].strip()
            paths.append(path)
        elif path and (l.startswith("+") or l.startswith("-")) and not (l.startswith("+++") or l.startswith("---")):
            t = l[1:].strip()
            if sum(c.isalnum() for c in t) >= 4:
                out.append((path, t))
    return out, paths


def kept(lines, out):
    if not lines:
        return None
    return sum(1 for _, t in lines if t in out or (len(t) >= 60 and t[:60] in out)) / len(lines)


def rows_for(repo, n, named):
    shas = sh(["git", "log", "--no-merges", f"-{n}", "--format=%H"], repo).split()
    cases = []
    if named:
        cases.append(("git diff HEAD~3", ["git", "diff", "HEAD~3"], ["git", "diff", "HEAD~3"]))
    for s in shas:
        cases.append((f"show {s[:7]}", ["git", "show", s], ["git", "show", s]))
        cases.append((f"diff {s[:7]}", ["git", "diff", f"{s}~1", s], ["git", "diff", f"{s}~1", s]))
    for name, cmd, rtk_cmd in cases:
        raw = sh(cmd, repo)
        if "diff --git" not in raw:
            continue
        outs = {"raw": raw}
        for k, b in BIN.items():
            outs[k] = sh([b] + rtk_cmd if k == "rtk" else [b, "run", "--"] + cmd, repo)
        lines, paths = changed(raw)
        src = [x for x in lines if not GENERATED.search(x[0])]
        r = {"name": f"{os.path.basename(repo)}:{name}", "raw_tok": tok(raw)}
        for k, o in outs.items():
            r[k] = (tok(o), kept(lines, o), kept(src, o), sum(1 for p in paths if p in o) / max(len(paths), 1), len(lines), len(src))
        yield r


def bucket(t):
    return "small" if t <= 400 else "medium" if t <= 4000 else "large"


def report(rows):
    def fmt(x):
        return "  -  " if x is None else f"{100 * x:5.1f}"
    groups = {}
    for r in rows:
        groups.setdefault(bucket(r["raw_tok"]), []).append(r)
    groups["ALL"] = rows
    print(f"\n{'bucket':8} {'n':>3} {'tool':9} {'tokens':>8} {'med-tok':>8} {'keep%':>6} {'pooled':>6} {'src%':>6} {'pooled':>6} {'files%':>6}")
    for b in ["small", "medium", "large", "ALL"]:
        g = groups.get(b, [])
        if not g:
            continue
        for k in TOOLS:
            toks = [r[k][0] for r in g]
            ks = [r[k][1] for r in g if r[k][1] is not None]
            ss = [r[k][2] for r in g if r[k][2] is not None]
            def pooled(i, w):
                xs = [(r[k][i], r[k][w]) for r in g if r[k][i] is not None]
                return sum(a * n for a, n in xs) / max(sum(n for _, n in xs), 1)
            pool, spool = pooled(1, 4), pooled(2, 5)
            fs = statistics.mean(r[k][3] for r in g)
            print(f"{b:8} {len(g):3} {k:9} {sum(toks):8} {int(statistics.median(toks)):8} {fmt(statistics.median(ks) if ks else None)} "
                  f"{fmt(pool)} {fmt(statistics.median(ss) if ss else None)} {fmt(spool)} {fmt(fs)}")
        print()


def main(argv):
    n, repos, show_rows = 40, [], "--rows" in argv
    i = 0
    while i < len(argv):
        if argv[i] == "--commits":
            n = int(argv[i + 1]); i += 1
        elif argv[i] == "--repo":
            repos.append(os.path.abspath(argv[i + 1])); i += 1
        i += 1
    repos = repos or [os.path.abspath(os.path.join(HERE, ".."))]
    rows = []
    for j, repo in enumerate(repos):
        for r in rows_for(repo, n, j == 0):
            rows.append(r)
            if show_rows or r["name"].endswith("HEAD~3"):
                print(f"{r['name']:28} raw={r['raw_tok']:6} " + " ".join(
                    f"{k}={r[k][0]}/{'-' if r[k][1] is None else round(100 * r[k][1])}%" for k in TOOLS if k != "raw"), flush=True)
    report(rows)


if __name__ == "__main__":
    main(sys.argv[1:])
