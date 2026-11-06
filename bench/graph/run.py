"""Vtok (`map`, `refs`) vs graphify (AST graph, no LLM) on the same questions. Needs graphify installed separately
(`pip install ./graphify` in its own venv) and graphs built with `graphify update DIR` on copies of the two trees.
Tokens: tiktoken o200k_base (proxy). No model in the loop.

Part 1 (orientation): for each "where is X handled" question, run `graphify query "<question>"` (its documented interface) and
check whether the target file / symbol shows up; also the size of GRAPH_REPORT.md (its orientation document). Compare with the
file/symbol hit rates of `vtok map` (bench/map).
Part 2 (callers): for names in `requests`, who uses NAME? Truth = Python's `ast` (functions whose body mentions NAME as a call or attribute).
Compare tokens and recall of `graphify explain NAME --all` and `vtok refs NAME`."""
import argparse, ast, json, pathlib, re, subprocess, sys
import tiktoken

ENC = tiktoken.get_encoding("o200k_base")
REPO = pathlib.Path(__file__).resolve().parents[2]
ntok = lambda s: len(ENC.encode(s))
sys.path.insert(0, str(REPO / "bench/map"))
import run as mapbench  # noqa: E402  (reuses the question list)


def sh(cmd, cwd):
    r = subprocess.run(cmd, cwd=cwd, capture_output=True, text=True)
    return r.stdout + r.stderr


def truth_callers(root, name):
    out = set()
    for f in pathlib.Path(root).rglob("*.py"):
        try: tree = ast.parse(f.read_text())
        except Exception: continue
        for fn in ast.walk(tree):
            if isinstance(fn, (ast.FunctionDef, ast.AsyncFunctionDef)):
                for n in ast.walk(fn):
                    if (isinstance(n, ast.Name) and n.id == name and isinstance(n.ctx, ast.Load)) or (isinstance(n, ast.Attribute) and n.attr == name):
                        if fn.name != name: out.add(fn.name)
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("out")
    ap.add_argument("--vtok", default=str(REPO / "rust/target/release/vtok"))
    ap.add_argument("--graphify", required=True)
    ap.add_argument("--vtok-copy", required=True, help="dir that holds the copy of rust/ with graphify-out/")
    ap.add_argument("--requests-copy", required=True)
    ap.add_argument("--gbudget", default="700")
    a = ap.parse_args()
    G = a.graphify
    res = {"orientation": [], "callers": []}
    copies = {"vtok": pathlib.Path(a.vtok_copy), "requests": pathlib.Path(a.requests_copy)}
    for tname, (root, _, qs) in mapbench.TREES.items():
        cp = copies[tname]
        report = (cp / "graphify-out/GRAPH_REPORT.md").read_text()
        for q, f, sym in qs:
            out = sh([G, "query", q, "--budget", a.gbudget], cp)
            vf = sh([a.vtok, "find", q, "--dir", str(root), "--max-tokens", "700"], REPO)
            vm = sh([a.vtok, "map", str(root), "--max-tokens", "700"], REPO)
            pm = mapbench.parse_map(vm)
            vfile = f in vf
            sname = sym.split(".")[-1] if sym else None
            res["orientation"].append({"tree": tname, "question": q, "file": f, "symbol": sym, "tokens": ntok(out), "file_hit": pathlib.Path(f).name in out,
                                       "symbol_hit": (sname in out) if sym else None, "report_has_file": pathlib.Path(f).name in report,
                                       "report_has_symbol": (sname in report) if sym else None,
                                       "find_tokens": ntok(vf), "find_file_hit": vfile, "find_symbol_hit": (any(l.startswith(f + " ") and sname in l for l in vf.splitlines())) if sym else None,
                                       "map_tokens": ntok(vm), "map_file_hit": f in pm, "map_symbol_hit": (sname in pm.get(f, "")) if sym else None})
        res["orientation"].append({"tree": tname, "question": "(report)", "report_tokens": ntok(report)})
    names = ["Session", "prepare_request", "merge_setting", "HTTPAdapter", "PreparedRequest", "urlparse", "get_adapter", "RequestException", "resolve_redirects", "select_proxy"]
    rcp = copies["requests"]
    for n in names:
        truth = truth_callers(rcp, n)
        g = sh([G, "explain", n, "--all"], rcp)
        gcallers = set(re.findall(r"<-- (\w+)\(\) \[calls\]", g))
        v = sh([a.vtok, "refs", n, str(rcp), "--max-tokens", "800"], REPO)
        vcallers = set()
        for l in v.splitlines():
            m = re.match(r"use \S+ (\S+) ", l)
            if m and m.group(1) not in ("imports", "(top"): vcallers.add(m.group(1).split(".")[-1])
        res["callers"].append({"name": n, "truth": sorted(truth), "graphify_tokens": ntok(g), "vtok_tokens": ntok(v),
                               "graphify_recall": len(truth & gcallers) / max(1, len(truth)), "vtok_recall": len(truth & vcallers) / max(1, len(truth)),
                               "graphify_found": sorted(gcallers), "vtok_found": sorted(vcallers)})
    pathlib.Path(a.out).write_text(json.dumps(res, indent=1))
    print("== orientation (graphify query, budget 700) ==")
    for tname in copies:
        rows = [r for r in res["orientation"] if r["tree"] == tname and r["question"] != "(report)"]
        rep = [r for r in res["orientation"] if r["tree"] == tname and r["question"] == "(report)"][0]
        syms = [r for r in rows if r["symbol"]]
        print(f"{tname}: {len(rows)} q; query output median {sorted(r['tokens'] for r in rows)[len(rows)//2]} tok; file named {sum(r['file_hit'] for r in rows)}/{len(rows)}, symbol named {sum(r['symbol_hit'] for r in syms)}/{len(syms)}; "
              f"GRAPH_REPORT.md {rep['report_tokens']} tok names file {sum(r['report_has_file'] for r in rows)}/{len(rows)}, symbol {sum(r['report_has_symbol'] for r in syms)}/{len(syms)}")
    print("\n== per question: graphify query | vtok find | vtok map (budget 700; F = target file named, S = target symbol named) ==")
    print("%-9s %-38s %-9s %-9s %-9s" % ("tree", "question", "graphify", "vtok find", "vtok map"))
    mark = lambda tok, f, sy: "%4d %s%s" % (tok, "F" if f else "-", "S" if sy else ("-" if sy is False else "."))
    for r in res["orientation"]:
        if r["question"] == "(report)": continue
        print("%-9s %-38s %-9s %-9s %-9s" % (r["tree"], r["question"][:38], mark(r["tokens"], r["file_hit"], r["symbol_hit"]), mark(r["find_tokens"], r["find_file_hit"], r["find_symbol_hit"]), mark(r["map_tokens"], r["map_file_hit"], r["map_symbol_hit"])))
    allq = [r for r in res["orientation"] if r["question"] != "(report)"]
    sq = [r for r in allq if r["symbol"]]
    print("%-48s %-9s %-9s %-9s" % ("TOTAL: tokens | files named | symbols named", "%d|%d|%d" % (sum(r["tokens"] for r in allq), sum(r["file_hit"] for r in allq), sum(r["symbol_hit"] for r in sq)),
          "%d|%d|%d" % (sum(r["find_tokens"] for r in allq), sum(r["find_file_hit"] for r in allq), sum(bool(r["find_symbol_hit"]) for r in sq)),
          "%d|%d|%d" % (sum(r["map_tokens"] for r in allq), sum(r["map_file_hit"] for r in allq), sum(bool(r["map_symbol_hit"]) for r in sq))))
    print("== callers (requests) ==")
    print("%-18s %6s %9s %9s %9s %9s" % ("name", "truth", "gfy tok", "vtok tok", "gfy rec", "vtok rec"))
    for r in res["callers"]:
        print("%-18s %6d %9d %9d %9.2f %9.2f" % (r["name"], len(r["truth"]), r["graphify_tokens"], r["vtok_tokens"], r["graphify_recall"], r["vtok_recall"]))
    c = res["callers"]
    print("mean recall: graphify %.2f, vtok %.2f; tokens: graphify %d, vtok %d" % (sum(r["graphify_recall"] for r in c) / len(c), sum(r["vtok_recall"] for r in c) / len(c), sum(r["graphify_tokens"] for r in c), sum(r["vtok_tokens"] for r in c)))


if __name__ == "__main__":
    main()
