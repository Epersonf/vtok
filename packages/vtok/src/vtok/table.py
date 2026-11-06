"""Alias table text format (shared by all languages): one `alias=original` per line."""

def render_table(table):
    """table: {original: alias}"""
    return "\n".join(f"{a}={o}" for o, a in sorted(table.items(), key=lambda kv: kv[1]))

def parse_table(text):
    """-> {alias: original}"""
    t = {}
    for ln in text.splitlines():
        if ln.strip():
            a, o = ln.split("=", 1)
            t[a.strip()] = o.strip()
    return t
