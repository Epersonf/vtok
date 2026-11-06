import pytest
from kv import parse_kv

def test_plain():
    assert parse_kv("a=1; b=two") == {"a": "1", "b": "two"}

def test_quoted():
    assert parse_kv('a="x;y=z"; b=2') == {"a": "x;y=z", "b": "2"}

def test_empty_and_spaces():
    assert parse_kv(" ;a = 1 ;; b= \"q r\" ;") == {"a": "1", "b": "q r"}

def test_missing_equals():
    with pytest.raises(ValueError) as e:
        parse_kv("a=1; oops")
    assert str(e.value) == "oops"
