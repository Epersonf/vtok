from collections import defaultdict, deque


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
