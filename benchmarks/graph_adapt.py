"""Load vtok (.vtok/graph.json, v2 or v3) and Graphify (graphify-out/graph.json) graphs into one shape and map their
nodes onto ground-truth definitions (graph_truth.py).

Shape: nodes = [{"file", "name", "a", "b", "kind"}] with kind in fn/type/value (code symbols) or None (files, docs,
external concepts); edges = [(src, dst, relation, cls, score)] with cls EXTRACTED or INFERRED.
"""
import json
import re

from graph_truth import norm_path

VTOK_KIND = {"function": "fn", "method": "fn", "type": "type", "value": "value"}
V2_REL = {"fn": "calls", "type": "uses_type", "value": "references"}
# relations that state a symbol-level use (what the ground truth measures); the rest are reported only as counts
SYMBOL_RELS = {"calls", "uses_type", "references", "uses"}


def load_vtok(path, sub):
    g = json.load(open(path, encoding="utf-8"))
    nodes = [{"file": norm_path(n[0], sub), "name": n[1].split(".")[-1], "a": n[2], "b": n[3],
              "kind": VTOK_KIND.get(n[4])} for n in g["nodes"]]
    edges = []
    rels = g.get("rels")
    for e in g["edges"]:
        if g.get("v", 1) >= 3:
            rel, score = rels[e[2]], e[3] / 100
            cls = "EXTRACTED" if e[3] >= 100 else "INFERRED"
        else:
            k = nodes[e[1]]["kind"]
            rel = V2_REL.get(k, "mentions") if nodes[e[0]]["kind"] else "mentions"
            cls = "INFERRED" if len(e) > 2 and e[2] else "EXTRACTED"
            score = 1.0 if cls == "EXTRACTED" else None
        edges.append((e[0], e[1], rel, cls, score))
    return nodes, edges


def load_graphify(path, sub):
    g = json.load(open(path, encoding="utf-8"))
    ids, nodes = {}, []
    for n in g["nodes"]:
        loc = re.match(r"L(\d+)", n.get("source_location") or "")
        label = n.get("label", "")
        code = n.get("file_type") == "code" and loc and not n.get("type") and not re.search(r"\.\w+$", label)
        name = label.strip().lstrip(".").removesuffix("()").split(".")[-1].split("::")[-1]
        kind = None
        if code:
            kind = "fn" if label.endswith(")") else "value" if re.fullmatch(r"[A-Z0-9_]+", name) else "type"
        ids[n["id"]] = len(nodes)
        line = int(loc.group(1)) if loc else 0
        nodes.append({"file": norm_path(n.get("source_file") or "", sub), "name": name, "a": line, "b": line, "kind": kind})
    edges = []
    for l in g.get("links") or g.get("edges"):
        s, t = ids.get(l["source"]), ids.get(l["target"])
        if s is None or t is None:
            continue
        rel = {"references": "uses"}.get(l["relation"], l["relation"])
        edges.append((s, t, rel, l.get("confidence", "EXTRACTED"), l.get("confidence_score")))
    return nodes, edges


def map_nodes(nodes, defs):
    """{node index: truth symbol}. Same file and name, and the definition line inside the node's line range (Graphify
    nodes carry only the name line: exact line first, then the nearest line within 3)."""
    by = {}
    for d in defs.values():
        by.setdefault((d["file"], d["name"]), []).append(d)
    out = {}
    for i, n in enumerate(nodes):
        if not n["kind"]:
            continue
        cands = by.get((n["file"], n["name"]), [])
        inside = [d for d in cands if n["a"] <= d["line"] <= n["b"]]
        if not inside and n["a"] == n["b"]:
            inside = [d for d in cands if abs(d["line"] - n["a"]) <= 3]
        if inside:
            out[i] = min(inside, key=lambda d: abs(d["line"] - n["a"]))["sym"]
    return out
