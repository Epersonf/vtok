"""TypeScript source -> Vtok-TS (tree-sitter CST walk).

The notation comes from `emit` (native) or `emitter` (keyword style); `analysis` counts the names to alias and
`literals` hides long strings."""
from vtok.errors import VtokError
from vtok.aliasing import plan_aliases
from .analysis import _excluded_and_counts
from .canon import canon
from .emit import Ctx, native_lines
from .emitter import Emitter
from .grammar import RESERVED
from .literals import collect_literals
from .parse import parse, first_error

def encode(source, alias=True, verify=True, keywords=True, table=None, view="full", style="native", hide=0, lits_out=None):
    source = source.replace("\r\n", "\n")
    root = parse(source.encode()).root_node
    err = first_error(root)
    if err is not None:
        raise VtokError(f"source has syntax errors near line {err.start_point[0] + 1}")
    if table is not None:
        table = dict(table)
    elif alias:
        counts, excl, avoid = _excluded_and_counts(root)
        bad = [n for n in counts if n in RESERVED and n in excl]
        if bad: raise VtokError(f"unsupported: imported/exported name collides with a Vtok token: {bad}")
        table = plan_aliases(counts, avoid=avoid, reserved=RESERVED, excluded=excl)
    else:
        table = {}
        counts, excl, _ = _excluded_and_counts(root)
        bad = [n for n in counts if n in RESERVED]
        if bad: table = plan_aliases({n: 1 for n in bad}, avoid=set(), reserved=RESERVED)
    lits = {}
    pos = {}
    if hide:
        pos, lits = collect_literals(root, hide)
        if lits_out is not None: lits_out.update(lits)
    if style == "native":
        vt = "\n".join(native_lines(root.named_children, Ctx(table, False, view == "skeleton", view == "lean", pos))) + "\n"
    else:
        em = Emitter(Ctx(table, keywords, view == "skeleton"))
        for c in root.named_children: em.stmt(c, 0)
        vt = "\n".join(em.lines) + "\n"
    if verify and view != "lean":   # lean output (import{names}) is intentionally not valid TS
        from .decode import decode
        back = decode(vt, {a: o for o, a in table.items()}, style=style, literals=lits)
        if view == "full" and canon(parse(back.encode()).root_node) != canon(root):
            raise VtokError("roundtrip verification failed (tree differs)")
    return vt, table
