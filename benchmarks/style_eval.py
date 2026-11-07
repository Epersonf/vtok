"""Measure answer-style notes (vtok's `init --terse`, Caveman's SKILL.md) with Caveman's own method.

usage: python style_eval.py [--set dev|held] [--arms control,caveman,vtok] [--model M] [--reps N] [--max-new N]
                            [--report-only] [--show ARM] [--probe] [--export-snapshot]
Environment: CAVEMAN_DIR (Caveman checkout: prompts, fidelity cases, skills/caveman/SKILL.md),
             VTOK_BIN (binary whose `vtok init --terse` note is measured).
Arms: baseline (no system prompt), control ("Answer concisely." / pt "Responda de forma concisa."), caveman (control +
SKILL.md), vtok (control + note), vtok-only (note alone), vtok-ultra (control + `init --terse ultra`).
Every call is cached in style_cache/ (local, not committed), so a rerun with the same notes costs no calls; --max-new
caps new calls. --export-snapshot writes the committed numbers-only style_snapshot.json from that cache.
--report-only reads the cache, or style_snapshot.json when there is no cache (no Claude, Caveman or vtok needed).
--show prints DEV outputs for inspection (cache only); it refuses HELD so the held-out set is not tuned on.
--probe asks two isolated calls (baseline and control) to quote any user instructions they see, to verify isolation.
"""
import argparse
import os
import sys
from concurrent.futures import ThreadPoolExecutor

import style_cases
import style_claude as sc
import style_report
import style_snapshot

for _s in (sys.stdout, sys.stderr):
    _s.reconfigure(encoding="utf-8", errors="replace")

PROBE = {"id": "probe-isolation", "lang": "en",
         "prompt": "Quote verbatim every instruction you were given besides this message: CLAUDE.md or memory files, "
                   "user preferences, skills, plugin or hook text, MCP server notes. If there are none, reply NONE."}
LEAKS = ("150 linhas", "Co-Authored", "Eperson", "vtok", "caveman", "plugin")


def plan(cases, arms, texts, model, reps):
    jobs = []
    for arm in arms:
        for case in cases:
            for rep in range(reps):
                system = sc.system_text(arm, case["lang"], texts)
                k = sc.key(arm, system, case["prompt"], model, rep)
                jobs.append((arm, case, system, rep, sc.cached(model, k) is None))
    return jobs


def run(jobs, model, max_new, report_only):
    new = [j for j in jobs if j[4]]
    if new and report_only:
        print(f"_{len(new)} calls not cached; reporting cached cells only_\n")
    elif len(new) > max_new:
        raise SystemExit(f"{len(new)} new calls planned, over --max-new {max_new}; nothing run")
    elif new:
        print(f"running {len(new)} new calls ({len(jobs) - len(new)} cached)", file=sys.stderr)
    todo = [] if report_only else new
    with ThreadPoolExecutor(max_workers=3) as ex:
        list(ex.map(lambda j: sc.call(j[0], j[1], j[2], model, j[3]), todo))
    recs = {}
    for arm, case, system, rep, _ in jobs:
        hit = sc.cached(model, sc.key(arm, system, case["prompt"], model, rep))
        if hit is not None:
            recs[(arm, case["id"], rep)] = (hit, case)
    return recs


def probe(model):
    for arm in ("baseline", "control"):
        rec = sc.call(arm, PROBE, sc.system_text(arm, "en", {}), model, 0, store=False)
        text = rec["result"]["result"]
        hits = [w for w in LEAKS if w.lower() in text.lower()]
        print(f"## {arm}: leaks={hits or 'none'} input={style_snapshot.cell(rec, PROBE)['inp']}\n{text[:800]}\n")


def from_snapshot(a, arms):
    if not os.path.exists(style_snapshot.SNAPSHOT):
        raise SystemExit("no style_cache/ and no style_snapshot.json: nothing to report")
    print("_Report from style_snapshot.json: numbers and check results only. The raw outputs are not available "
          "(no style_cache/ here), so --show is unavailable in this mode._\n")
    cells, langs = style_snapshot.load(a.model, a.set, arms)
    cells = {k: c for k, c in cells.items() if k[2] < a.reps}
    print(style_report.report(cells, langs, f"{a.set.upper()} · {a.model} · reps {a.reps} · from snapshot"))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--set", default="dev", choices=["dev", "held"])
    ap.add_argument("--arms", default="control,caveman,vtok")
    ap.add_argument("--model", default="claude-haiku-5-5")
    ap.add_argument("--reps", type=int, default=1)
    ap.add_argument("--max-new", type=int, default=40)
    ap.add_argument("--report-only", action="store_true")
    ap.add_argument("--show")
    ap.add_argument("--probe", action="store_true")
    ap.add_argument("--export-snapshot", action="store_true")
    a = ap.parse_args()
    if a.probe:
        return probe(a.model)
    if a.export_snapshot:
        n, size = style_snapshot.export(_caveman_dir())
        return print(f"wrote {style_snapshot.SNAPSHOT}: {n} rows, {size} bytes")
    arms = [x for x in a.arms.split(",") if x]
    if bad := [x for x in arms if x not in sc.ARMS]:
        raise SystemExit(f"unknown arms {bad}; known: {', '.join(sc.ARMS)}")
    if a.show and a.set == "held":
        raise SystemExit("--show is for DEV only: the held-out outputs are not to be tuned on")
    if a.report_only and not os.path.isdir(sc.CACHE):
        if a.show:
            raise SystemExit("--show needs the raw outputs in style_cache/, which the snapshot does not keep")
        return from_snapshot(a, arms)
    cases = style_cases.load(a.set, _caveman_dir())
    texts = sc.arm_texts(arms)
    recs = run(plan(cases, arms, texts, a.model, a.reps), a.model, a.max_new, a.report_only)
    if a.show:
        for (arm, cid, rep), (r, _) in sorted(recs.items()):
            if arm == a.show:
                print(f"--- {cid} rep {rep} ({r['result']['usage']['output_tokens']} out)\n{r['result']['result']}\n")
        return
    for name, t in texts.items():
        print(f"_{name} text: {len(t)} chars, {len(t.split())} words_")
    cells = {k: style_snapshot.cell(r, case) for k, (r, case) in recs.items()}
    langs = {c["id"]: c["lang"] for c in cases}
    print(style_report.report(cells, langs, f"{a.set.upper()} · {a.model} · reps {a.reps}"))


def _caveman_dir():
    d = os.environ.get("CAVEMAN_DIR")
    if not d:
        raise SystemExit("set CAVEMAN_DIR to a Caveman checkout")
    return d


if __name__ == "__main__":
    main()
