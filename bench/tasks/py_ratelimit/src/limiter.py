"""Sliding-window rate limiter."""
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
