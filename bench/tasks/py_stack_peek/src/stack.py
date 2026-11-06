"""A tiny stack."""

class Stack:
    """LIFO container."""

    def __init__(self):
        self._items = []

    def push(self, item):
        """Push an item."""
        self._items.append(item)

    def pop(self):
        """Remove and return the top item."""
        return self._items.pop()

    def size(self):
        return len(self._items)
