"""Chunking support for the TypeScript plugin."""
from vtok.language import Unit
from vtok.errors import VtokError
from .parse import parse, first_error
from .canon import canon
from .decode import decode as _decode

WRAP = "__vtok_wrap__"
CLASS_DECLS = ("class_declaration", "abstract_class_declaration")

def _name(n):
    d = n.child_by_field_name("declaration") if n.type == "export_statement" else n
    d = d if d is not None else n
    nm = d.child_by_field_name("name")
    if nm is not None:
        return nm.text.decode()
    if d.type in ("lexical_declaration", "variable_declaration"):
        names = []
        for vd in d.named_children:
            if vd.type == "variable_declarator":
                x = vd.child_by_field_name("name")
                if x is not None: names.append(x.text.decode())
        if names: return ",".join(names)
    if d.type == "import_statement": return "imports"
    return d.type

def _rng(n): return n.start_point[0] + 1, n.end_point[0] + 1

def units(source, max_lines):
    root = parse(source.replace("\r\n", "\n").encode()).root_node
    out = []
    for n in root.named_children:
        if n.type == "comment": continue
        s, e = _rng(n)
        d = n.child_by_field_name("declaration") if n.type == "export_statement" else n
        if d is not None and d.type in CLASS_DECLS and e - s + 1 > max_lines:
            body = d.child_by_field_name("body")
            cname = _name(n)
            for m in body.named_children:
                if m.type == "comment": continue
                ms, me = _rng(m)
                out.append(Unit(_name(m) if m.child_by_field_name("name") is not None else m.type, m.type, ms, me, True, cname, m.start_point[1]))
        else:
            out.append(Unit(_name(n), n.type, s, e, False, None, n.start_point[1]))
    return out

def _wrapped(text, member):
    return f"class {WRAP} {{\n{text}\n}}" if member else text

def encode_chunk(lang, text, table, member):
    if not member:
        return lang.encode(text, table=table).vtok
    from .encode import native_lines, Ctx
    root = parse(_wrapped(text, True).encode()).root_node
    body = root.named_children[0].child_by_field_name("body")
    return "\n".join(native_lines(body.named_children, Ctx(table, False))) + "\n"

def decode_chunk(vtok, inv, member, indent, original=None):
    if not member:
        return _decode(vtok, inv)
    out = _decode(f"class {WRAP}{{\n{vtok}\n}}\n", inv)
    root = parse(out.encode()).root_node
    body = root.named_children[0].child_by_field_name("body")
    pad = " " * indent
    return "".join(pad + m.text.decode() + "\n" for m in body.named_children if m.type != "comment")

def equivalent(a, b, member):
    try:
        ta, tb = parse(_wrapped(a, member).encode()).root_node, parse(_wrapped(b, member).encode()).root_node
    except Exception:
        return False
    if first_error(ta) or first_error(tb): return False
    return canon(ta) == canon(tb)

def validate(source):
    err = first_error(parse(source.encode()).root_node)
    if err is not None:
        raise VtokError(f"result is not valid TypeScript near line {err.start_point[0] + 1}", err.start_point[0] + 1)
