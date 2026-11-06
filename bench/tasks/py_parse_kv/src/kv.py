"""Key/value parsing."""

def parse_kv(text):
    """Parse 'a=1; b=two' into a dict."""
    out = {}
    for part in text.split(";"):
        part = part.strip()
        if not part:
            continue
        key, value = part.split("=", 1)
        out[key.strip()] = value.strip()
    return out
