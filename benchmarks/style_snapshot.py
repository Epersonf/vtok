"""Compact, committed evidence for style_eval.py: benchmarks/style_snapshot.json, built from the local style_cache/.

The cache holds raw `claude -p` results (prompt-side system text, answers, session ids) and stays out of git. The
snapshot keeps only numbers and check results per call: model, set, arm, case id, language, rep, output and thinking
tokens, input-side tokens (input + cache creation + cache read) and what the arm adds over the control, cost, answer
length in characters, turns, each check's pass flag and the overall pass. No prompt, answer or system text, no session
ids, no paths. vtok's own note text is stored (it is ours); Caveman's SKILL.md only as sha256 + length + its path.
"""
import glob
import hashlib
import json
import os
import re

import style_cases
import style_claude as sc

SNAPSHOT = os.path.join(sc.HERE, "style_snapshot.json")
CAVEMAN_SKILL = "skills/caveman/SKILL.md"
# Nothing personal or machine-specific may reach the committed file.
FORBIDDEN = [r"epersoncmxf", r"gmail", r"epers", r"[\w.+-]+@[\w-]+\.[\w.]+", r"[A-Za-z]:\\\\", r"/Users/", r"AppData",
             r"\"session_id\"", r"\"uuid\""]


def sha(text):
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def style_text(rec):
    """The text an arm adds on top of the control (note or skill), or None for baseline/control."""
    sysx, arm = rec["system"], rec["arm"]
    if arm in ("baseline", "control") or sysx is None:
        return None
    return sysx if arm == "vtok-only" else sysx.split("\n\n", 1)[1]


def cell(rec, case):
    """The compact row for one cached call."""
    r = rec["result"]
    u = r["usage"]
    checks = case.get("checks")
    flags = None
    if checks:
        inc = [bool(re.search(p, r["result"], re.I)) for p in checks.get("must_include", [])]
        exc = [not re.search(p, r["result"], re.I) for p in checks.get("must_not_include", [])]
        flags = {"must_include": inc, "must_not_include": exc}
    return {"out": u["output_tokens"], "think": (u.get("output_tokens_details") or {}).get("thinking_tokens") or 0,
            "inp": u["input_tokens"] + u["cache_creation_input_tokens"] + u["cache_read_input_tokens"],
            "cost": r["total_cost_usd"], "chars": len(r["result"]), "turns": r.get("num_turns", 1),
            "checks": flags, "failed": style_cases.check(r["result"], checks) if checks else None}


def scrub_check(text):
    hits = [p for p in FORBIDDEN if re.search(p, text, re.I)]
    if hits:
        raise SystemExit(f"snapshot not written: it matches forbidden patterns {hits}")


def export(caveman_dir):
    cases = {c["id"]: dict(c, set=s) for s in ("dev", "held") for c in style_cases.load(s, caveman_dir)}
    note, skill = sc.vtok_note(), sc.caveman_skill()
    rows = []
    for f in sorted(glob.glob(os.path.join(sc.CACHE, "*", "*.json"))):
        with open(f, encoding="utf-8") as fh:
            rec = json.load(fh)
        case = cases.get(rec["case"])
        if case is None:
            continue
        st = style_text(rec)
        rows.append(dict({"model": rec["model"], "set": case["set"], "arm": rec["arm"], "case": rec["case"],
                          "lang": case["lang"], "rep": rec["rep"], "style_sha": st and sha(st)}, **cell(rec, case)))
    ctl = {(r["model"], r["case"], r["rep"]): r["inp"] for r in rows if r["arm"] == "control"}
    for r in rows:
        k = (r["model"], r["case"], r["rep"])
        r["inp_added"] = r["inp"] - ctl[k] if k in ctl and r["arm"] != "control" else None
        r["pass"] = None if r["failed"] is None else not r["failed"]
        r["failed"] = None if r["failed"] is None else len(r["failed"])  # count only: the messages quote patterns
    meta = {"isolation": sc.ISOLATION, "control": sc.CONTROL,
            "vtok_note": {"sha256": sha(note), "chars": len(note), "text": note},
            "caveman_skill": {"sha256": sha(skill), "chars": len(skill), "path": CAVEMAN_SKILL},
            "cases": {cid: {"set": c["set"], "lang": c["lang"], "checks": c.get("checks")} for cid, c in cases.items()}}
    text = json.dumps({"meta": meta, "rows": rows}, ensure_ascii=False, separators=(",", ":"))
    text = text.replace('},{"model"', '},\n{"model"')  # one row per line: readable diffs
    scrub_check(text)
    with open(SNAPSHOT, "w", encoding="utf-8", newline="\n") as f:
        f.write(text + "\n")
    return len(rows), os.path.getsize(SNAPSHOT)


def load(model, split, arms):
    """(cells, langs) for style_report from the committed snapshot; vtok/caveman rows only for the recorded texts."""
    with open(SNAPSHOT, encoding="utf-8") as f:
        snap = json.load(f)
    meta = snap["meta"]
    want = {"vtok": meta["vtok_note"]["sha256"], "vtok-only": meta["vtok_note"]["sha256"],
            "caveman": meta["caveman_skill"]["sha256"]}
    cells = {}
    for r in snap["rows"]:
        if r["model"] != model or r["set"] != split or r["arm"] not in arms:
            continue
        if r["arm"] in want and r["style_sha"] != want[r["arm"]]:
            continue  # an earlier draft of the note
        failed = None if r["pass"] is None else _failed(meta["cases"][r["case"]]["checks"], r["checks"])
        cells[(r["arm"], r["case"], r["rep"])] = dict(r, failed=failed)
    langs = {cid: c["lang"] for cid, c in meta["cases"].items() if c["set"] == split}
    return cells, langs


def _failed(checks, flags):
    bad = [f"missing /{p}/" for p, ok in zip(checks.get("must_include", []), flags["must_include"]) if not ok]
    return bad + [f"forbidden /{p}/" for p, ok in zip(checks.get("must_not_include", []), flags["must_not_include"]) if not ok]
