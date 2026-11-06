import pytest
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
