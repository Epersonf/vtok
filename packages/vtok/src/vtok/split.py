"""Split big files into addressable chunks; edit one chunk at a time; splice back (spec 6).

index  = skeleton view of the whole file + chunk map (what the LLM reads first)
chunk  = full lossless Vtok of one line range (what the LLM fetches / edits)
apply  = decode edited chunks and splice them into the ORIGINAL file by line range,
         leaving every other chunk (comments, formatting) byte-identical.
"""
from dataclasses import dataclass
from .errors import VtokError
from .project import plan_project

@dataclass
class Chunk:
    id: str
    label: str
    start: int
    end: int
    member: bool
    indent: int

    @property
    def lines(self): return self.end - self.start + 1

def plan_chunks(lang, source, max_lines=150):
    n = len(source.splitlines())
    if n <= max_lines:
        return [Chunk("c1", "(whole file)", 1, n, False, 0)]
    groups = []   # [parent, start, end, names, indent, member]
    for u in lang.units(source, max_lines):
        g = groups[-1] if groups else None
        if g and g[0] == u.parent and (u.start <= g[2] or u.end - g[1] + 1 <= max_lines):
            g[2] = max(g[2], u.end); g[3].append(u.name)
        else:
            groups.append([u.parent, u.start, u.end, [u.name], u.indent, u.member])
    out = []
    for i, (parent, a, b, names, ind, member) in enumerate(groups, 1):
        uniq = list(dict.fromkeys(names))
        lab = " ".join(uniq[:6]) + (f" +{len(uniq) - 6}" if len(uniq) > 6 else "")
        out.append(Chunk(f"c{i}", (f"{parent}: " if parent else "") + lab, a, b, member, ind))
    return out

def file_table(lang, source, table=None):
    return table if table is not None else plan_project(lang, [source])

def chunk_source(source, c):
    return "".join(source.splitlines(keepends=True)[c.start - 1:c.end])

def render_index(lang, source, chunks, table):
    """Skeleton Vtok + chunk map. For a small file (single chunk) the caller should use the full view."""
    from .table import render_table
    sk = lang.encode(source, table=table, view="skeleton").vtok
    rows = []
    for c in chunks:
        lab = " ".join(table.get(w, w) for w in c.label.replace(":", ": ").split())
        rows.append(f"{c.id} L{c.start}-{c.end} {lab}")
    return sk + "\n# chunks (fetch one by id)\n" + "\n".join(rows) + "\n"

def chunk_vtok(lang, source, chunk, table):
    return lang.encode_chunk(chunk_source(source, chunk), table, chunk.member)

def apply_edits(lang, source, chunks, edits, table, force=False):
    """edits: {chunk_id: vtok}. Returns new source. Unchanged chunks are not touched."""
    by_id = {c.id: c for c in chunks}
    inv = {a: o for o, a in table.items()}
    lines = source.splitlines(keepends=True)
    todo = []
    for cid, vt in edits.items():
        if cid not in by_id:
            raise VtokError(f"unknown chunk {cid!r}; known: {sorted(by_id)}")
        c = by_id[cid]
        orig = chunk_source(source, c)
        new = lang.decode_chunk(vt, inv, c.member, c.indent, orig)
        if not force and lang.equivalent(orig, new, c.member):
            continue
        todo.append((c, new))
    for c, new in sorted(todo, key=lambda t: -t[0].start):
        lines[c.start - 1:c.end] = [new if new.endswith("\n") else new + "\n"]
    out = "".join(lines)
    lang.validate(out)
    return out
