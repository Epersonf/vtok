"""Precision / recall of a graph's symbol edges against the SCIP ground truth (graph_truth.py, graph_adapt.py)."""
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


def evaluate(nodes, edges, truth, with_tests):
    defs, tedges, impls = truth
    keep = {s for s, d in defs.items() if with_tests or not d["test"]}
    gold = {c: {k for k, v in tedges.items() if v == c and k[0] in keep and k[1] in keep} for c in CATS}
    m = map_nodes(nodes, defs)
    covered = {s for s in m.values() if s in keep}
    out = {"coverage": len(covered) / max(1, len(keep)), "defs": len(keep),
           "coverage_by_kind": {k: (sum(1 for s in covered if defs[s]["kind"] == k), sum(1 for s in keep if defs[s]["kind"] == k)) for k in ("fn", "type", "value")}}

    def is_test(i):
        return defs[m[i]]["test"] if i in m else looks_test(nodes[i])

    pred = {}
    for s, t, rel, cls, score in edges:
        if rel not in SYMBOL_RELS or not nodes[s]["kind"] or not nodes[t]["kind"]:
            continue
        if not with_tests and (is_test(s) or is_test(t)):
            continue
        a, b = m.get(s, ("unmapped", s)), m.get(t, ("unmapped", t))
        if a == b:
            continue
        cat = EDGE_KIND[defs[b]["kind"]] if t in m else EDGE_KIND[nodes[t]["kind"]]
        k = (a, b, cat)
        if k not in pred or cls == "EXTRACTED":
            pred[k] = cls
    for c in CATS:
        mine = {k: v for k, v in pred.items() if k[2] == c}
        tp = {k for k in mine if (k[0], k[1]) in gold[c]}
        p, r, f = prf(len(tp), len(mine), len(gold[c]))
        row = {"pred": len(mine), "truth": len(gold[c]), "tp": len(tp), "precision": p, "recall": r, "f1": f}
        for cls in ("EXTRACTED", "INFERRED"):
            sub = [k for k, v in mine.items() if v == cls]
            hit = sum(1 for k in sub if k in tp)
            row[cls] = {"pred": len(sub), "tp": hit, "precision": hit / len(sub) if sub else 0.0, "recall": hit / max(1, len(gold[c]))}
        out[c] = row
    imp = {(m.get(s), m.get(t)) for s, t, rel, _, _ in edges if rel == "implements"}
    gimp = {k for k in impls if k[0] in keep and k[1] in keep}
    out["implements"] = {"pred": len(imp), "truth": len(gimp), "tp": len(imp & gimp)}
    out["relations"] = dict(Counter(f"{rel} {cls}" for _, _, rel, cls, _ in edges).most_common())
    return out


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
