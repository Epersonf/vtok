"""Task `py_flatten` (python, target flat.py): the instruction, the project to change, its tests and a reference solution."""
TASK = dict(lang="python", target="flat.py", cmd="py", instruction=
 "Add `flatten(d, sep='.')` to the module: flattens nested dicts into one dict with joined keys "
 "({'a': {'b': 1, 'c': {'d': 2}}, 'e': 3} -> {'a.b': 1, 'a.c.d': 2, 'e': 3}); non-dict values (including lists) are kept as-is; "
 "an empty nested dict is kept as an empty dict under its key. Also add `unflatten(d, sep='.')` as the inverse, and keep `get_path` unchanged.",
 src={"flat.py": '''"""Nested dict helpers."""

def get_path(d, path, default=None):
    """Read d['a']['b'] via 'a.b'."""
    cur = d
    for part in path.split("."):
        if not isinstance(cur, dict) or part not in cur:
            return default
        cur = cur[part]
    return cur
'''},
 tests={"test_flat.py": '''from flat import flatten, unflatten, get_path

def test_flatten():
    assert flatten({"a": {"b": 1, "c": {"d": 2}}, "e": 3}) == {"a.b": 1, "a.c.d": 2, "e": 3}

def test_sep_lists_empty():
    assert flatten({"a": {"b": [1, {"x": 1}]}, "z": {}}, sep="/") == {"a/b": [1, {"x": 1}], "z": {}}

def test_unflatten():
    assert unflatten({"a.b": 1, "a.c.d": 2, "e": 3}) == {"a": {"b": 1, "c": {"d": 2}}, "e": 3}

def test_roundtrip():
    d = {"a": {"b": {"c": 1}}, "k": 2}
    assert unflatten(flatten(d)) == d

def test_get_path():
    assert get_path({"a": {"b": 5}}, "a.b") == 5 and get_path({}, "a.b", 9) == 9
'''},
 solution={"flat.py": '''def get_path(d, path, default=None):
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
'''})
