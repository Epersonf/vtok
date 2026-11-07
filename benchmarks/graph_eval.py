"""Edge accuracy of `vtok graph` and Graphify against a compiler-grade ground truth (rust-analyzer SCIP).

Prerequisites (nothing is downloaded): `rustup component add rust-analyzer`, the Graphify CLI on PATH (or GRAPHIFY),
and a built vtok (`cargo build --release -p vtok-cli`, or VTOK). Python 3.10+ stdlib only.

    python benchmarks/graph_eval.py            # evaluates the repo's own rust/ workspace

Environment (all optional):
  EVAL_SRC      Cargo workspace to evaluate (default: <repo>/rust); EVAL_SUB the graphed subdirectory (default crates)
  EVAL_EXTRA    comma list of other top-level entries the workspace needs to load (members, build.rs), copied unmeasured
  EVAL_WORK     scratch directory for the copies and outputs (default: <tmp>/vtok-graph-eval)
  VTOK          vtok binary (default: <repo>/rust/target/release/vtok[.exe], else `vtok` on PATH)
  GRAPHIFY      graphify binary (default `graphify`); set GRAPHIFY=skip to leave it out
  RUST_ANALYZER rust-analyzer binary (default `rust-analyzer`); EVAL_REUSE=1 reuses an existing index.scip
  EVAL_JSON     write the full results there as JSON
Every tool runs on its own copy of the code inside EVAL_WORK, never inside the repository.
"""
import json
import os
import shutil
import subprocess
import sys
import tempfile
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import graph_adapt  # noqa: E402
import graph_metrics  # noqa: E402
import graph_truth  # noqa: E402

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


def env(name, default):
    return os.environ.get(name) or default


def copy_workspace(src, dst, sub):
    if os.path.isdir(dst):
        shutil.rmtree(dst)
    os.makedirs(dst)
    ignore = shutil.ignore_patterns("target", ".vtok", "graphify-out", "node_modules", ".git")
    # EVAL_EXTRA: other workspace members or files the manifest needs (e.g. "engine,text,plugin,build.rs").
    for f in ["Cargo.toml", "Cargo.lock"] + [x for x in os.environ.get("EVAL_EXTRA", "").split(",") if x]:
        p = os.path.join(src, f)
        if os.path.isdir(p):
            shutil.copytree(p, os.path.join(dst, f), ignore=ignore)
        elif os.path.exists(p):
            shutil.copy(p, dst)
    shutil.copytree(os.path.join(src, sub), os.path.join(dst, sub), ignore=ignore)


def timed(cmd, **kw):
    t = time.perf_counter()
    r = subprocess.run(cmd, capture_output=True, text=True, encoding="utf-8", errors="replace", **kw)
    if r.returncode != 0:
        raise SystemExit(f"{' '.join(cmd)} failed:\n{r.stdout[-2000:]}\n{r.stderr[-2000:]}")
    return time.perf_counter() - t, r.stdout


def vtok_bin():
    exe = os.path.join(REPO, "rust", "target", "release", "vtok" + (".exe" if os.name == "nt" else ""))
    return env("VTOK", exe if os.path.exists(exe) else "vtok")


def tokens(text):
    return round(len(text) / 4)


def main():
    src = env("EVAL_SRC", os.path.join(REPO, "rust"))
    sub = env("EVAL_SUB", "crates")
    work = env("EVAL_WORK", os.path.join(tempfile.gettempdir(), "vtok-graph-eval"))
    ra = os.path.join(work, "truth")
    index = os.path.join(ra, "index.scip")
    if not (os.environ.get("EVAL_REUSE") == "1" and os.path.exists(index)):
        if os.path.abspath(src) != os.path.abspath(ra):
            copy_workspace(src, ra, sub)
        dt, _ = timed([env("RUST_ANALYZER", "rust-analyzer"), "scip", "."], cwd=ra, env={**os.environ, "CARGO_NET_OFFLINE": "true"})
        print(f"rust-analyzer scip: {dt:.1f} s", file=sys.stderr)
    truth = graph_truth.load(index, ra, sub)
    tools, facts = {}, {}
    vt = os.path.join(work, "vtok")
    copy_workspace(src, vt, sub)
    dt, _ = timed([vtok_bin(), "graph", "build", "--dir", os.path.join(vt, sub)])
    gj = os.path.join(vt, sub, ".vtok", "graph.json")
    _, rep = timed([vtok_bin(), "graph", "report", "--dir", os.path.join(vt, sub)])
    tools["vtok"] = graph_adapt.load_vtok(gj, sub)
    facts["vtok"] = {"build_s": round(dt, 2), "graph_kb": round(os.path.getsize(gj) / 1024), "report_tokens": tokens(rep)}
    gf_bin = env("GRAPHIFY", "graphify")
    if gf_bin != "skip":
        gf = os.path.join(work, "graphify")
        copy_workspace(src, gf, sub)
        dt, _ = timed([gf_bin, "update", os.path.join(gf, sub)])
        out = os.path.join(gf, sub, "graphify-out")
        tools["graphify"] = graph_adapt.load_graphify(os.path.join(out, "graph.json"), sub)
        rep = open(os.path.join(out, "GRAPH_REPORT.md"), encoding="utf-8").read()
        facts["graphify"] = {"build_s": round(dt, 2), "graph_kb": round(os.path.getsize(os.path.join(out, "graph.json")) / 1024), "report_tokens": tokens(rep)}
    defs, edges, impls = truth
    print(f"# graph eval: {sum(1 for d in defs.values() if not d['test'])} non-test definitions ({len(defs)} with tests), "
          f"{len(edges)} truth edges, {len(impls)} implements\n")
    results = {}
    for mode, with_tests in (("non-test code", False), ("all code (tests included)", True)):
        res = {t: graph_metrics.evaluate(n, e, truth, with_tests) for t, (n, e) in tools.items()}
        results[mode] = res
        print(graph_metrics.table(res, mode))
    print("| tool | nodes | edges | graph.json | build | report (~tokens) |\n|---|---|---|---|---|---|")
    for t, (n, e) in tools.items():
        f = facts[t]
        print(f"| {t} | {len(n)} | {len(e)} | {f['graph_kb']} KB | {f['build_s']} s | {f['report_tokens']} |")
    for t, r in results["non-test code"].items():
        print(f"\n{t} relations: " + ", ".join(f"{k} {v}" for k, v in r["relations"].items()))
    if os.environ.get("EVAL_JSON"):
        with open(os.environ["EVAL_JSON"], "w", encoding="utf-8") as fh:
            json.dump({"results": results, "facts": facts}, fh, indent=1)


if __name__ == "__main__":
    main()
