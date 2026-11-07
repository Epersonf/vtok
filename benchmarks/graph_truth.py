"""Ground-truth definitions and edges for Rust from a rust-analyzer SCIP index (see scip_read.py).

A definition is a workspace function/method, type (struct, enum, union, trait, type alias) or value (const, static).
An edge A -> B is recorded for every reference occurrence of B that lies inside the enclosing range of A (the innermost
such definition in the same document); the kind follows B: call (fn/method), type_use (type), reference (value).
Implements edges come from impl symbols `impl#[Type][Trait]`.
Test code: a definition inside a module whose name starts with `test` (this covers `#[cfg(test)] mod tests` and
`#[path = "x_tests.rs"] mod tests`), in a file under `tests/` or `benches/`, or marked `#[test]`.
"""
import os
import re

from scip_read import DEFINITION, read_index

FN, TYPE, VALUE = {17, 26, 70, 80}, {11, 49, 53, 55, 61}, {8, 82}
KIND = {**{k: "fn" for k in FN}, **{k: "type" for k in TYPE}, **{k: "value" for k in VALUE}}
EDGE_KIND = {"fn": "call", "type": "type_use", "value": "reference"}


def last_name(sym):
    """Name of the last descriptor of a SCIP symbol: `a/b/impl#[T]f().` -> f, `a/T#` -> T."""
    body = re.sub(r"\[[^\]]*\]", "", sym.split(" ", 4)[-1])
    parts = [p for p in re.split(r"[/#.()!]+", body) if p]
    return parts[-1].strip("`") if parts else ""


def is_test(sym, path, lines, line):
    segs = sym.split(" ", 4)[-1].split("/")[:-1]
    if any(s.startswith("test") for s in segs) or re.search(r"(^|/)(tests|benches)/", path):
        return True
    above = " ".join(lines[max(0, line - 4):line])
    return "#[test]" in above or "#[cfg(test)]" in above


def norm_path(p, sub):
    p = p.replace("\\", "/")
    return p[len(sub) + 1:] if sub and p.startswith(sub + "/") else p


def load(index_path, src_root, sub="crates"):
    """Returns (defs: {symbol: def dict}, edges: {(symA, symB): kind}, implements: {(type symbol, trait symbol)})."""
    docs = read_index(index_path)
    kinds = {s["symbol"]: s["kind"] for d in docs for s in d["symbols"]}
    # Only the graphed subdirectory counts: other workspace members are indexed but no tool is asked to read them.
    docs = [d for d in docs if not sub or d["path"].replace("\\", "/").startswith(sub + "/")]
    defs, per_doc, impls = {}, {}, set()
    for d in docs:
        path = norm_path(d["path"], sub)
        try:
            with open(os.path.join(src_root, d["path"]), encoding="utf-8", errors="replace") as fh:
                lines = fh.read().split("\n")
        except OSError:
            lines = []
        for o in d["occurrences"]:
            s = o["symbol"]
            if not (o["roles"] & DEFINITION) or s.startswith("local"):
                continue
            k = KIND.get(kinds.get(s, 0))
            if not k or "impl#[" in s.rsplit("/", 1)[-1] and s.endswith("]"):
                continue
            ext = o["enclosing"] or o["range"]
            line = o["range"][0] + 1
            dd = {"sym": s, "file": path, "name": last_name(s), "line": line, "start": ext[:2], "end": ext[2:],
                  "kind": k, "test": is_test(s, path, lines, line)}
            defs[s] = dd
            per_doc.setdefault(d["path"], []).append(dd)
    edges = {}
    for d in docs:
        own = per_doc.get(d["path"], [])
        heads = impl_heads(src_root, d["path"])
        for o in d["occurrences"]:
            s = o["symbol"]
            if o["roles"] & DEFINITION or s not in defs:
                continue
            ln, col = o["range"][:2]
            if ln in heads and defs[s]["kind"] == "type":
                heads[ln].append((col, s))
            pos = o["range"][:2]
            inner = [x for x in own if x["start"] <= pos <= x["end"]]
            if not inner:
                continue
            a = max(inner, key=lambda x: x["start"])["sym"]
            if a != s:
                edges[(a, s)] = EDGE_KIND[defs[s]["kind"]]
        for ln, occ in heads.items():
            cut = occ.pop(0)
            tr = [s for c, s in occ if c < cut]
            ty = [s for c, s in occ if c > cut]
            if tr and ty:
                impls.add((ty[0], tr[-1]))
    return defs, edges, impls


def impl_heads(src_root, rel):
    """{0-based line: [(column of ' for ', ...)]} for `impl Trait for Type` header lines of a file."""
    out = {}
    try:
        with open(os.path.join(src_root, rel), encoding="utf-8", errors="replace") as fh:
            for i, l in enumerate(fh.read().split("\n")):
                m = re.match(r"\s*(?:unsafe\s+)?impl\b.*?\sfor\s", l)
                if m:
                    out[i] = [m.end() - 4]
    except OSError:
        pass
    return out
