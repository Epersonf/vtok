"""The `vtok` command line (argparse). The commands are `cmd_codec` (notation) and `cmd_chunks` (big files); `cli_common` has what they share."""
import argparse, sys
from .cmd_chunks import cmd_apply, cmd_chunk, cmd_split
from .cmd_codec import cmd_decode, cmd_encode, cmd_grammar, cmd_languages, cmd_roundtrip, cmd_stats

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
