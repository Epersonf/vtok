from collections import OrderedDict


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
