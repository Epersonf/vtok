from flat import flatten, unflatten, get_path

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
