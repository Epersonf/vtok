"""`vtok languages | encode | decode | stats | roundtrip | grammar`: the notation itself, file by file."""
import sys
from . import render_table, parse_table, VtokError, ntok, registry
from .cli_common import _lang, _files
from .tokens import BACKEND

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

def cmd_grammar(a): print(registry.get_language(a.lang).grammar_prompt)
