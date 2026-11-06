def get_path(d, path, default=None):
    cur = d
    for part in path.split("."):
        if not isinstance(cur, dict) or part not in cur:
            return default
        cur = cur[part]
    return cur


def flatten(d, sep="."):
    out = {}
    for key, value in d.items():
        if isinstance(value, dict) and value:
            for k, v in flatten(value, sep).items():
                out[key + sep + k] = v
        else:
            out[key] = value
    return out


def unflatten(d, sep="."):
    out = {}
    for key, value in d.items():
        cur = out
        parts = key.split(sep)
        for part in parts[:-1]:
            cur = cur.setdefault(part, {})
        cur[parts[-1]] = value
    return out
