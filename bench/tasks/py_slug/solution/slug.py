import re


def slugify(text, max_len=None):
    out = re.sub(r"[^a-z0-9]+", "-", text.lower()).strip("-")
    if max_len is not None:
        out = out[:max_len].rstrip("-")
    return out
