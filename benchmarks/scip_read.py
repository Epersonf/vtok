"""Minimal, stdlib-only reader for SCIP index files (protobuf wire format).

Only the subset graph_eval needs is decoded:
  Index{metadata=1, documents=2, external_symbols=3}
  Document{relative_path=1, occurrences=2, symbol_information=3, language=4}
  Occurrence{range=1 packed int32, symbol=2, symbol_roles=3, syntax_kind=5, enclosing_range=7 packed int32}
  SymbolInformation{symbol=1, documentation=3, relationships=4, kind=5, display_name=6}
  Relationship{symbol=1, is_reference=2, is_implementation=3, is_type_definition=4, is_definition=5}
Field numbers follow scip.proto; `python scip_read.py index.scip` prints a sample to check them on real data.
"""
import sys

DEFINITION = 0x1


def varint(buf, i):
    shift = out = 0
    while True:
        b = buf[i]
        i += 1
        out |= (b & 0x7F) << shift
        if b < 0x80:
            return out, i
        shift += 7


def fields(buf):
    """Yield (field number, wire type, value) for one message; value is int or a memoryview slice."""
    i, n = 0, len(buf)
    while i < n:
        key, i = varint(buf, i)
        f, wt = key >> 3, key & 7
        if wt == 0:
            v, i = varint(buf, i)
        elif wt == 2:
            ln, i = varint(buf, i)
            v, i = buf[i:i + ln], i + ln
        elif wt == 1:
            v, i = int.from_bytes(buf[i:i + 8], "little"), i + 8
        elif wt == 5:
            v, i = int.from_bytes(buf[i:i + 4], "little"), i + 4
        else:
            raise ValueError(f"unsupported wire type {wt}")
        yield f, wt, v


def packed(v, wt):
    if wt == 0:
        return [v]
    out, i = [], 0
    while i < len(v):
        x, i = varint(v, i)
        out.append(x)
    return out


def norm_range(r):
    """[startLine, startChar, endLine, endChar] (0-based) from SCIP's 3- or 4-element form."""
    return [r[0], r[1], r[0], r[2]] if len(r) == 3 else list(r[:4])


def occurrence(buf):
    o = {"range": [], "symbol": "", "roles": 0, "enclosing": None}
    for f, wt, v in fields(buf):
        if f == 1:
            o["range"] += packed(v, wt)
        elif f == 2:
            o["symbol"] = bytes(v).decode("utf-8", "replace")
        elif f == 3:
            o["roles"] = v
        elif f == 7:
            o["enclosing"] = (o["enclosing"] or []) + packed(v, wt)
    o["range"] = norm_range(o["range"])
    if o["enclosing"]:
        o["enclosing"] = norm_range(o["enclosing"])
    return o


def symbol_info(buf):
    s = {"symbol": "", "kind": 0, "display": "", "impls": []}
    for f, wt, v in fields(buf):
        if f == 1:
            s["symbol"] = bytes(v).decode("utf-8", "replace")
        elif f == 5:
            s["kind"] = v
        elif f == 6:
            s["display"] = bytes(v).decode("utf-8", "replace")
        elif f == 4:
            rel = {k: val for k, _, val in fields(v)}
            if rel.get(3) and 1 in rel:
                s["impls"].append(bytes(rel[1]).decode("utf-8", "replace"))
    return s


def document(buf):
    d = {"path": "", "occurrences": [], "symbols": [], "language": ""}
    for f, _, v in fields(buf):
        if f == 1:
            d["path"] = bytes(v).decode("utf-8", "replace")
        elif f == 2:
            d["occurrences"].append(occurrence(v))
        elif f == 3:
            d["symbols"].append(symbol_info(v))
        elif f == 4:
            d["language"] = bytes(v).decode("utf-8", "replace")
    return d


def read_index(path):
    with open(path, "rb") as fh:
        buf = memoryview(fh.read())
    return [document(v) for f, _, v in fields(buf) if f == 2]


if __name__ == "__main__":
    docs = read_index(sys.argv[1])
    occ = sum(len(d["occurrences"]) for d in docs)
    print(f"{len(docs)} documents, {occ} occurrences")
    d = max(docs, key=lambda d: len(d["occurrences"]))
    print(d["path"], d["language"])
    for o in d["occurrences"][:8]:
        print(" ", o)
    for s in d["symbols"][:8]:
        print(" ", s)
