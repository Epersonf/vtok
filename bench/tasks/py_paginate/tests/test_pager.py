import pytest
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
