"""Precision / recall of a graph's symbol edges against the SCIP ground truth (graph_truth.py, graph_adapt.py)."""
import os
from collections import Counter

from graph_adapt import SYMBOL_RELS, map_nodes
from graph_truth import EDGE_KIND

CATS = ("call", "type_use", "reference")


def looks_test(n):
    base = n["file"].rsplit("/", 1)[-1]
    return "test" in base or n["name"].startswith("test")


def prf(tp, npred, ntruth):
    p = tp / npred if npred else 0.0
    r = tp / ntruth if ntruth else 0.0
    return p, r, (2 * p * r / (p + r) if p + r else 0.0)


def evaluate(nodes, edges, truth, with_tests, strict_map=os.environ.get("EVAL_UNMAPPED") != "fp"):
    defs, tedges, impls = truth
    keep = {s for s, d in defs.items() if with_tests or not d["test"]}
    gold = {c: {k for k, v in tedges.items() if v == c and k[0] in keep and k[1] in keep} for c in CATS}
    m = map_nodes(nodes, defs)
    covered = {s for s in m.values() if s in keep}
    out = {"coverage": len(covered) / max(1, len(keep)), "defs": len(keep),
           "coverage_by_kind": {k: (sum(1 for s in covered if defs[s]["kind"] == k), sum(1 for s in keep if defs[s]["kind"] == k)) for k in ("fn", "type", "value")}}

    def is_test(i):
        return defs[m[i]]["test"] if i in m else looks_test(nodes[i])

    # files rust-analyzer did not index (e.g. behind a cfg it did not enable) cannot be judged either way
    indexed = {d["file"] for d in defs.values()}
    pred, unjudged = {}, 0
    for s, t, rel, cls, score in edges:
        if rel not in SYMBOL_RELS or not nodes[s]["kind"] or not nodes[t]["kind"]:
            continue
        # an end the index has no definition for (a file or item behind a disabled cfg, e.g. `#[cfg(unix)]` on
        # Windows or an optional feature) cannot be judged; counted per tool as "unjudged"
        if nodes[s]["file"] not in indexed or nodes[t]["file"] not in indexed or (strict_map and (s not in m or t not in m)):
            unjudged += 1
            continue
        if not with_tests and (is_test(s) or is_test(t)):
            continue
        a, b = m.get(s, ("unmapped", s)), m.get(t, ("unmapped", t))
        if a == b:
            continue
        cat = EDGE_KIND[defs[b]["kind"]] if t in m else EDGE_KIND[nodes[t]["kind"]]
        k = (a, b, cat)
        sc = 1.0 if score is None and cls == "EXTRACTED" else (score or 0.0)
        if k not in pred or sc > pred[k][1]:
            pred[k] = (cls, sc)
    for c in CATS:
        mine = {k: v[0] for k, v in pred.items() if k[2] == c}
        tp = {k for k in mine if (k[0], k[1]) in gold[c]}
        p, r, f = prf(len(tp), len(mine), len(gold[c]))
        row = {"pred": len(mine), "truth": len(gold[c]), "tp": len(tp), "precision": p, "recall": r, "f1": f}
        for cls in ("EXTRACTED", "INFERRED"):
            sub = [k for k, v in mine.items() if v == cls]
            hit = sum(1 for k in sub if k in tp)
            row[cls] = {"pred": len(sub), "tp": hit, "precision": hit / len(sub) if sub else 0.0, "recall": hit / max(1, len(gold[c]))}
        out[c] = row
    out["unjudged"] = unjudged
    out["calib"] = {c: calibrate({k: v[1] for k, v in pred.items() if k[2] == c}, gold[c]) for c in CATS}
    imp = {(m.get(s), m.get(t)) for s, t, rel, _, _ in edges if rel == "implements" and nodes[s]["file"] in indexed and nodes[t]["file"] in indexed}
    gimp = {k for k in impls if k[0] in keep and k[1] in keep}
    out["implements"] = {"pred": len(imp), "truth": len(gimp), "tp": len(imp & gimp)}
    out["relations"] = dict(Counter(f"{rel} {cls}" for _, _, rel, cls, _ in edges).most_common())
    return out


BUCKETS = ((1.0, "1.0"), (0.9, "0.90-0.99"), (0.8, "0.80-0.89"), (0.7, "0.70-0.79"), (0.6, "0.60-0.69"), (0.0, "<0.60"))


def calibrate(scored, gold):
    """{bucket label: [predicted, true positives]} for edges keyed (a, b, cat) with a score."""
    out = {lab: [0, 0] for _, lab in BUCKETS}
    for k, sc in scored.items():
        lab = next(lab for lo, lab in BUCKETS if sc >= lo - 1e-9)
        out[lab][0] += 1
        out[lab][1] += (k[0], k[1]) in gold
    return out


def calib_table(results, title):
    """Precision per score bucket and relation category: the scores should match these numbers."""
    lines = [f"### calibration, {title}", "", "| tool | category | " + " | ".join(lab for _, lab in BUCKETS) + " |", "|---|---|" + "---|" * len(BUCKETS)]
    for tool, r in results.items():
        for c in CATS:
            cells = [f"{100 * tp / n:.0f}% of {n}" if n else "-" for n, tp in (r["calib"][c][lab] for _, lab in BUCKETS)]
            lines.append(f"| {tool} | {c} | " + " | ".join(cells) + " |")
    return "\n".join(lines) + "\n"


def table(results, title):
    """results: {tool: evaluate() output}. A compact markdown table."""
    lines = [f"### {title}", "", "| tool | node cov | calls P | calls R | calls F1 | type P | type R | ref P | ref R | impl tp/pred/truth |",
             "|---|---|---|---|---|---|---|---|---|---|"]
    pc = lambda x: f"{100 * x:.1f}%"
    for tool, r in results.items():
        c, t, v, i = r["call"], r["type_use"], r["reference"], r["implements"]
        lines.append(f"| {tool} | {pc(r['coverage'])} | {pc(c['precision'])} | {pc(c['recall'])} | {pc(c['f1'])} | {pc(t['precision'])} | {pc(t['recall'])} | {pc(v['precision'])} | {pc(v['recall'])} | {i['tp']}/{i['pred']}/{i['truth']} |")
    lines += ["", "| tool | class | calls pred | calls P | calls R | type pred | type P | type R |", "|---|---|---|---|---|---|---|---|"]
    for tool, r in results.items():
        for cls in ("EXTRACTED", "INFERRED"):
            c, t = r["call"][cls], r["type_use"][cls]
            lines.append(f"| {tool} | {cls} | {c['pred']} | {pc(c['precision'])} | {pc(c['recall'])} | {t['pred']} | {pc(t['precision'])} | {pc(t['recall'])} |")
    return "\n".join(lines) + "\n"
