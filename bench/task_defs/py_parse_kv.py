"""Task `py_parse_kv` (python, target kv.py): the instruction, the project to change, its tests and a reference solution."""
TASK = dict(lang="python", target="kv.py", cmd="py", instruction=
 "`parse_kv(text)` parses 'a=1; b=two' into a dict. Extend it so values may be double-quoted and then may contain ';' and '=' "
 "(e.g. 'a=\"x;y=z\"; b=2' -> {'a': 'x;y=z', 'b': '2'}). Whitespace around keys and unquoted values is stripped; "
 "empty segments are ignored; a segment without '=' raises ValueError(segment).",
 src={"kv.py": '''"""Key/value parsing."""

def parse_kv(text):
    """Parse 'a=1; b=two' into a dict."""
    out = {}
    for part in text.split(";"):
        part = part.strip()
        if not part:
            continue
        key, value = part.split("=", 1)
        out[key.strip()] = value.strip()
    return out
'''},
 tests={"test_kv.py": '''import pytest
from kv import parse_kv

def test_plain():
    assert parse_kv("a=1; b=two") == {"a": "1", "b": "two"}

def test_quoted():
    assert parse_kv('a="x;y=z"; b=2') == {"a": "x;y=z", "b": "2"}

def test_empty_and_spaces():
    assert parse_kv(" ;a = 1 ;; b= \\"q r\\" ;") == {"a": "1", "b": "q r"}

def test_missing_equals():
    with pytest.raises(ValueError) as e:
        parse_kv("a=1; oops")
    assert str(e.value) == "oops"
'''},
 solution={"kv.py": '''def _segments(text):
    seg, quoted, out = [], False, []
    for ch in text:
        if ch == '"':
            quoted = not quoted
            seg.append(ch)
        elif ch == ";" and not quoted:
            out.append("".join(seg)); seg = []
        else:
            seg.append(ch)
    out.append("".join(seg))
    return out


def parse_kv(text):
    out = {}
    for part in _segments(text):
        part = part.strip()
        if not part:
            continue
        if "=" not in part:
            raise ValueError(part)
        key, value = part.split("=", 1)
        value = value.strip()
        if len(value) >= 2 and value[0] == '"' and value[-1] == '"':
            value = value[1:-1]
        out[key.strip()] = value
    return out
'''})
