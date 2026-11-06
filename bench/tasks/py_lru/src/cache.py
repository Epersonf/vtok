"""Tiny cache."""

class SimpleCache:
    """Key/value store with a nominal capacity."""

    def __init__(self, capacity):
        self.capacity = capacity
        self._data = {}

    def get(self, key, default=None):
        return self._data.get(key, default)

    def put(self, key, value):
        self._data[key] = value
