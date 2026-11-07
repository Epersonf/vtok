"""Task `py_paginate` (python, target pager.py): the instruction, the project to change, its tests and a reference solution."""
TASK = dict(lang="python", target="pager.py", cmd="py", instruction=
 "`paginate(items, page, size)` must return the items of the 1-based page `page` (page 1 = first `size` items). "
 "It currently returns the wrong slice. Also raise ValueError if page < 1 or size < 1.",
 src={"pager.py": '''"""Pagination helpers."""
# NOTE: pages are 1-based

def paginate(items, page, size):
    """Return one page of items."""
    start = page * size
    end = start + size
    return list(items[start:end])


def page_count(total, size):
    """Number of pages needed for `total` items."""
    return (total + size - 1) // size
'''},
 tests={"test_pager.py": '''import pytest
from pager import paginate, page_count

def test_first_page():
    assert paginate([1,2,3,4,5], 1, 2) == [1,2]

def test_last_partial():
    assert paginate([1,2,3,4,5], 3, 2) == [5]

def test_beyond():
    assert paginate([1,2,3], 5, 2) == []

def test_invalid():
    with pytest.raises(ValueError):
        paginate([1], 0, 2)
    with pytest.raises(ValueError):
        paginate([1], 1, 0)

def test_count():
    assert page_count(5, 2) == 3
'''},
 solution={"pager.py": '''def paginate(items, page, size):
    if page < 1 or size < 1:
        raise ValueError("page and size must be >= 1")
    start = (page - 1) * size
    return list(items[start:start + size])


def page_count(total, size):
    return (total + size - 1) // size
'''})
