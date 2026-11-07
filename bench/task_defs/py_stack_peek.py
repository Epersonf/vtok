"""Task `py_stack_peek` (python, target stack.py): the instruction, the project to change, its tests and a reference solution."""
TASK = dict(lang="python", target="stack.py", cmd="py", instruction=
 "Add to class `Stack`: a method `peek()` returning the top item without removing it (raise IndexError('empty') if empty), "
 "a method `is_empty()` returning a bool, and make `pop()` also raise IndexError('empty') when empty. "
 "Add `__len__` too.",
 src={"stack.py": '''"""A tiny stack."""

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
'''},
 tests={"test_stack.py": '''import pytest
from stack import Stack

def test_peek_and_len():
    s = Stack(); s.push(1); s.push(2)
    assert s.peek() == 2 and len(s) == 2 and not s.is_empty()

def test_empty_errors():
    s = Stack()
    assert s.is_empty()
    for f in (s.peek, s.pop):
        with pytest.raises(IndexError) as e:
            f()
        assert str(e.value) == "empty"

def test_size_kept():
    s = Stack(); s.push(1)
    assert s.size() == 1
'''},
 solution={"stack.py": '''class Stack:
    def __init__(self):
        self._items = []

    def push(self, item):
        self._items.append(item)

    def pop(self):
        if not self._items:
            raise IndexError("empty")
        return self._items.pop()

    def peek(self):
        if not self._items:
            raise IndexError("empty")
        return self._items[-1]

    def is_empty(self):
        return not self._items

    def size(self):
        return len(self._items)

    def __len__(self):
        return len(self._items)
'''})
