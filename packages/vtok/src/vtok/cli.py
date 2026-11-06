import argparse, pathlib, sys
from . import render_table, parse_table, VtokError, ntok, registry
from .tokens import BACKEND

def _lang(a, path=None):
    if getattr(a, "lang", None):
        return registry.get_language(a.lang)
    return registry.language_for_path(path)

def _files(paths, lang_name=None):
    exts = None
    if lang_name:
        exts = registry.get_language(lang_name).extensions
    for p in paths:
        p = pathlib.Path(p)
        if p.is_dir():
            for f in sorted(x for x in p.rglob("*") if x.is_file()):
                if (exts and f.suffix.lower() in exts) or (not exts and f.suffix.lower() in
                        {e for l in registry.available().values() for e in l.extensions}):
                    yield f
        else:
            yield p

def cmd_languages(a):
    langs = registry.available()
    for n, l in sorted(langs.items()):
        print(f"{n:12} {' '.join(l.extensions)}")
    for n, pkg in registry.KNOWN.items():
        if n not in langs:
            print(f"{n:12} (not installed: pip install {pkg})")
    for n, e in registry.broken().items():
        print(f"{n:12} BROKEN: {e}")

def cmd_encode(a):
    lang = _lang(a, a.file)
    r = lang.encode(open(a.file, encoding="utf-8").read(), alias=not a.no_alias, view=a.view, hide_literals=a.hide_literals)
    sys.stdout.write(r.vtok)
    if r.literals and a.literals:
        import json; json.dump(r.literals, open(a.literals, "w"))
    if r.table:
        sys.stderr.write("# alias table\n" + render_table(r.table) + "\n")
        if a.table: open(a.table, "w").write(render_table(r.table) + "\n")

def cmd_decode(a):
    lang = registry.get_language(a.lang)
    table = parse_table(open(a.table).read()) if a.table else None
    try:
        import json
        lits = json.load(open(a.literals)) if a.literals else None
        sys.stdout.write(lang.decode(open(a.file, encoding="utf-8").read(), table, lits))
    except VtokError as e:
        print(f"parse error: {e}", file=sys.stderr); return 1

def cmd_stats(a):
    tr = tn = tv = tt = n = 0
    print(f"tokenizer: {BACKEND}")
    print(f"{'file':40} {'raw':>7} {'nodoc':>7} {'vtok':>7} {'table':>6} {'net%':>6}")
    gp = 0; seen = set()
    for f in _files(a.paths, a.lang):
        try:
            lang = _lang(a, f); src = f.read_text(encoding="utf-8")
            r = lang.encode(src, alias=not a.no_alias, view=a.view)
            raw, norm = ntok(src), ntok(lang.normalize(src))
            v, t = ntok(r.vtok), ntok(render_table(r.table))
            if lang.name not in seen: seen.add(lang.name); gp += ntok(lang.grammar_prompt)
        except Exception as e:
            print(f"{str(f)[-40:]:40} SKIP ({type(e).__name__}: {str(e)[:40]})"); continue
        n += 1; tn += norm; tr += raw; tv += v; tt += t
        print(f"{str(f)[-40:]:40} {raw:7} {norm:7} {v:7} {t:6} {100*(1-(v+t)/max(raw,1)):6.1f}")
    if n:
        print(f"{'TOTAL (%d files)' % n:40} {tr:7} {tn:7} {tv:7} {tt:6} {100*(1-(tv+tt)/tr):6.1f}")
        print(f"syntax-only gain vs comment-free source: {100*(1-(tv+tt)/tn):.1f}%")
        print(f"net incl. one-time grammar prompt ({gp} tokens): {100*(1-(tv+tt+gp)/tr):.1f}%")

def cmd_roundtrip(a):
    ok = bad = 0
    for f in _files(a.paths, a.lang):
        try:
            lang = _lang(a, f); src = f.read_text(encoding="utf-8")
        except Exception as e:
            print(f"SKIP {f}: {e}"); continue
        try:
            lang.encode(src, alias=not a.no_alias, verify=True); ok += 1
        except Exception as e:
            bad += 1; print(f"FAIL {f}: {type(e).__name__}: {str(e)[:110]}")
    print(f"roundtrip ok={ok} fail={bad} ({100*ok/max(ok+bad,1):.1f}% ok)")
    return 1 if bad else 0

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

def cmd_grammar(a): print(registry.get_language(a.lang).grammar_prompt)

def main(argv=None):
    p = argparse.ArgumentParser(prog="vtok")
    sub = p.add_subparsers(dest="cmd", required=True)
    def lang_opt(x, req=False): x.add_argument("--lang", required=req, help="language plugin (default: by file extension)")
    s = sub.add_parser("languages"); s.set_defaults(fn=cmd_languages)
    e = sub.add_parser("encode"); e.add_argument("file"); lang_opt(e); e.add_argument("--table"); e.add_argument("--no-alias", action="store_true"); e.add_argument("--view", default="full", choices=["full", "skeleton", "lean"]); e.add_argument("--hide-literals", type=int, default=0, metavar="N", help="hide string literals of >= N tokens; needs --literals FILE to decode"); e.add_argument("--literals"); e.set_defaults(fn=cmd_encode)
    d = sub.add_parser("decode"); d.add_argument("file"); lang_opt(d, True); d.add_argument("--table"); d.add_argument("--literals"); d.set_defaults(fn=cmd_decode)
    s = sub.add_parser("stats"); s.add_argument("paths", nargs="+"); lang_opt(s); s.add_argument("--no-alias", action="store_true"); s.add_argument("--view", default="full", choices=["full", "skeleton"]); s.set_defaults(fn=cmd_stats)
    r = sub.add_parser("roundtrip"); r.add_argument("paths", nargs="+"); lang_opt(r); r.add_argument("--no-alias", action="store_true"); r.set_defaults(fn=cmd_roundtrip)
    for name, fn in (("split", cmd_split), ("chunk", cmd_chunk), ("apply", cmd_apply)):
        c = sub.add_parser(name); c.add_argument("file"); lang_opt(c)
        c.add_argument("--max-lines", type=int, default=150)
        if name == "chunk": c.add_argument("id")
        if name == "apply":
            c.add_argument("-e", "--edit", action="append", required=True, metavar="ID=FILE")
            c.add_argument("-o", "--output"); c.add_argument("--force", action="store_true")
        c.set_defaults(fn=fn)
    g = sub.add_parser("grammar"); lang_opt(g, True); g.set_defaults(fn=cmd_grammar)
    a = p.parse_args(argv)
    try:
        return a.fn(a) or 0
    except LookupError as e:
        print(f"error: {e}", file=sys.stderr); return 2
