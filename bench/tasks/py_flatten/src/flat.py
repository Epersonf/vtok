"""Nested dict helpers."""

def get_path(d, path, default=None):
    """Read d['a']['b'] via 'a.b'."""
    cur = d
    for part in path.split("."):
        if not isinstance(cur, dict) or part not in cur:
            return default
        cur = cur[part]
    return cur
