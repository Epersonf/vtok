"""`vtok split | chunk | apply`: one big file as chunks, one chunk, edited chunks spliced back."""
import sys
from . import render_table, VtokError, ntok
from .cli_common import _lang

def _chunks_for(a):
    from . import plan_chunks, file_table
    lang = _lang(a, a.file)
    src = open(a.file, encoding="utf-8").read().replace("\r\n", "\n")
    return lang, src, plan_chunks(lang, src, a.max_lines), file_table(lang, src)

def cmd_split(a):
    from . import render_index
    lang, src, chunks, table = _chunks_for(a)
    raw = ntok(src)
    if len(chunks) == 1:
        r = lang.encode(src, view="full", table=table)
        sys.stdout.write(r.vtok)
        print(f"# {len(src.splitlines())} lines <= {a.max_lines}: not split (c1 = whole file)", file=sys.stderr)
        return
    idx = render_index(lang, src, chunks, table)
    sys.stdout.write(idx)
    if table:
        sys.stderr.write("# alias table\n" + render_table(table) + "\n")
    print(f"# raw={raw} tokens; index={ntok(idx)} ({100*ntok(idx)/raw:.0f}%); {len(chunks)} chunks, "
          f"avg {sum(ntok(__import__('vtok').chunk_vtok(lang, src, c, table)) for c in chunks)//len(chunks)} tokens each",
          file=sys.stderr)

def cmd_chunk(a):
    from . import chunk_vtok
    lang, src, chunks, table = _chunks_for(a)
    by = {c.id: c for c in chunks}
    if a.id not in by:
        print(f"error: unknown chunk {a.id!r}; known: {', '.join(by)}", file=sys.stderr); return 2
    sys.stdout.write(chunk_vtok(lang, src, by[a.id], table))

def cmd_apply(a):
    from . import apply_edits
    lang, src, chunks, table = _chunks_for(a)
    edits = {}
    for spec in a.edit:
        cid, _, path = spec.partition("=")
        edits[cid] = open(path, encoding="utf-8").read()
    try:
        out = apply_edits(lang, src, chunks, edits, table, force=a.force)
    except VtokError as e:
        print(f"parse error: {e}", file=sys.stderr); return 1
    if a.output: open(a.output, "w", encoding="utf-8").write(out)
    else: sys.stdout.write(out)
