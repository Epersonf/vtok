"""Task `py_ratelimit` (python, target limiter.py): the instruction, the project to change, its tests and a reference solution."""
TASK = dict(lang="python", target="limiter.py", cmd="py", instruction=
 "Class `RateLimiter` allows `limit` calls per key within a sliding window of `window` seconds. Add: "
 "(1) method `reset(key)` that forgets a key's history; (2) an optional per-key override: `set_limit(key, limit)` so that "
 "`allow(key, now)` uses that key's limit instead of the default; (3) method `remaining(key, now)` returning how many more "
 "calls are allowed right now for that key (never negative). `allow` must keep its current behaviour otherwise.",
 src={"limiter.py": '''"""Sliding-window rate limiter."""
from collections import defaultdict, deque


class RateLimiter:
    """Allow `limit` calls per `window` seconds, per key."""

    def __init__(self, limit, window):
        self.limit = limit
        self.window = window
        self._hits = defaultdict(deque)

    def _prune(self, key, now):
        hits = self._hits[key]
        while hits and hits[0] <= now - self.window:
            hits.popleft()
        return hits

    def allow(self, key, now):
        """Record a call if allowed; return True when allowed."""
        hits = self._prune(key, now)
        if len(hits) >= self.limit:
            return False
        hits.append(now)
        return True
'''},
 tests={"test_limiter.py": '''from limiter import RateLimiter

def test_basic():
    r = RateLimiter(2, 10)
    assert r.allow("a", 0) and r.allow("a", 1) and not r.allow("a", 2)
    assert r.allow("a", 11)

def test_reset():
    r = RateLimiter(1, 10)
    assert r.allow("a", 0) and not r.allow("a", 1)
    r.reset("a")
    assert r.allow("a", 2)

def test_override():
    r = RateLimiter(1, 10)
    r.set_limit("vip", 3)
    assert all(r.allow("vip", t) for t in (0, 1, 2)) and not r.allow("vip", 3)
    assert r.allow("x", 0) and not r.allow("x", 1)

def test_remaining():
    r = RateLimiter(3, 10)
    assert r.remaining("a", 0) == 3
    r.allow("a", 0); r.allow("a", 1)
    assert r.remaining("a", 2) == 1
    r.allow("a", 2)
    assert r.remaining("a", 3) == 0
    assert r.remaining("a", 20) == 3
'''},
 solution={"limiter.py": '''from collections import defaultdict, deque


class RateLimiter:
    def __init__(self, limit, window):
        self.limit = limit
        self.window = window
        self._hits = defaultdict(deque)
        self._limits = {}

    def _prune(self, key, now):
        hits = self._hits[key]
        while hits and hits[0] <= now - self.window:
            hits.popleft()
        return hits

    def _limit_for(self, key):
        return self._limits.get(key, self.limit)

    def set_limit(self, key, limit):
        self._limits[key] = limit

    def reset(self, key):
        self._hits.pop(key, None)

    def remaining(self, key, now):
        return max(0, self._limit_for(key) - len(self._prune(key, now)))

    def allow(self, key, now):
        hits = self._prune(key, now)
        if len(hits) >= self._limit_for(key):
            return False
        hits.append(now)
        return True
'''})
