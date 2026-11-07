"""Prompt sets for style_eval.py: Caveman's eval prompts (en, pt-BR, fidelity cases) plus vtok's own dev-task cases,
split into DEV (used while tuning the terse note) and HELD (only for confirmation).

A case is {"id", "lang", "prompt", "checks"?}; checks follow Caveman's score_fidelity.py: every `must_include` regex
matches and no `must_not_include` regex does, case-insensitive. Cases without checks only count toward length.
The split is fixed by index here so it cannot drift while tuning. Two Caveman fidelity cases are left out because they
test Caveman's own on/off commands ("keeping caveman active", "stop caveman"), which mean nothing to other notes.
"""
import json
import os
import re

HERE = os.path.dirname(os.path.abspath(__file__))

# Indexes into Caveman's prompts/en.txt and pt.txt (pt mirrors en line by line).
EN = {"dev": [0, 1, 2], "held": [3, 4, 5]}
PT = {"dev": [6, 7, 8, 9], "held": [3, 4, 5]}
# Ids from Caveman's prompts/fidelity.json.
FID = {
    "dev": ["polarity-and-limits", "destructive-production-action", "language-preservation",
            "public-artifact", "sigkill-exit-code", "leaked-aws-key"],
    "held": ["never-force-push-main", "rotate-db-password-steps", "github-issue-report",
             "rate-limit-two-windows", "git-reset-soft-hard"],
}
SKIPPED_FID = ["quoted-deactivation", "stop-then-answer"]


def _lines(path):
    with open(path, encoding="utf-8") as f:
        return [l.strip() for l in f if l.strip()]


def _json(path):
    with open(path, encoding="utf-8") as f:
        return json.load(f)


def _lang_of(text):
    return "pt" if re.search(r"[ãõçáéêíóú]|\b(não|por que|como|qual|explique|pra)\b", text, re.I) else "en"


def load(split, caveman_dir):
    """All cases of `split` ("dev" or "held"), Caveman's first, each with a stable id."""
    ev = os.path.join(caveman_dir, "evals", "prompts")
    out = []
    for lang, idx in (("en", EN), ("pt", PT)):
        lines = _lines(os.path.join(ev, f"{lang}.txt"))
        out += [{"id": f"cave-{lang}-{i}", "lang": lang, "prompt": lines[i]} for i in idx[split]]
    fid = {c["id"]: c for c in _json(os.path.join(ev, "fidelity.json"))["cases"]}
    for cid in FID[split]:
        c = fid[cid]
        out.append({"id": f"fid-{cid}", "lang": _lang_of(c["prompt"]), "prompt": c["prompt"], "checks": c["checks"]})
    out += _json(os.path.join(HERE, f"style_cases_{split}.json"))["cases"]
    return out


def check(output, checks):
    """Failed checks for one output; empty means the case passed."""
    bad = [f"missing /{p}/" for p in checks.get("must_include", []) if not re.search(p, output, re.I)]
    bad += [f"forbidden /{p}/" for p in checks.get("must_not_include", []) if re.search(p, output, re.I)]
    return bad
