"""Task `py_lru` (python, target cache.py): the instruction, the project to change, its tests and a reference solution."""
TASK = dict(lang="python", target="cache.py", cmd="py", instruction=
 "`SimpleCache(capacity)` currently never evicts. Make it a least-recently-used cache: `get(key, default=None)` and `put(key, value)` "
 "both count as a use; when `put` would exceed `capacity`, evict the least recently used key. Add `__contains__` (does NOT count as a use), "
 "`__len__`, and `keys()` returning keys from least to most recently used.",
 src={"cache.py": '''"""Tiny cache."""

class SimpleCache:
    """Key/value store with a nominal capacity."""

    def __init__(self, capacity):
        self.capacity = capacity
        self._data = {}

    def get(self, key, default=None):
        return self._data.get(key, default)

    def put(self, key, value):
        self._data[key] = value
'''},
 tests={"test_cache.py": '''from cache import SimpleCache

def test_evicts_lru():
    c = SimpleCache(2)
    c.put("a", 1); c.put("b", 2); c.get("a"); c.put("c", 3)
    assert "b" not in c and "a" in c and "c" in c and len(c) == 2

def test_put_existing_refreshes():
    c = SimpleCache(2)
    c.put("a", 1); c.put("b", 2); c.put("a", 10); c.put("c", 3)
    assert c.get("a") == 10 and "b" not in c

def test_contains_is_not_a_use():
    c = SimpleCache(2)
    c.put("a", 1); c.put("b", 2)
    assert "a" in c
    c.put("c", 3)
    assert "a" not in c

def test_keys_order_and_default():
    c = SimpleCache(3)
    c.put("a", 1); c.put("b", 2); c.put("c", 3); c.get("a")
    assert c.keys() == ["b", "c", "a"] and c.get("zz", 7) == 7
'''},
 solution={"cache.py": '''from collections import OrderedDict


class SimpleCache:
    def __init__(self, capacity):
        self.capacity = capacity
        self._data = OrderedDict()

    def get(self, key, default=None):
        if key in self._data:
            self._data.move_to_end(key)
            return self._data[key]
        return default

    def put(self, key, value):
        self._data[key] = value
        self._data.move_to_end(key)
        while len(self._data) > self.capacity:
            self._data.popitem(last=False)

    def __contains__(self, key):
        return key in self._data

    def __len__(self):
        return len(self._data)

    def keys(self):
        return list(self._data)
'''})
