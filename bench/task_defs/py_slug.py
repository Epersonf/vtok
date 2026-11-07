"""Task `py_slug` (python, target slug.py): the instruction, the project to change, its tests and a reference solution."""
TASK = dict(lang="python", target="slug.py", cmd="py", instruction=
 "`slugify(text)` only lowercases. Make it: lowercase, replace every run of non-alphanumeric ASCII characters with a single '-', "
 "strip leading/trailing '-'. Add optional `max_len=None`: when given, cut the result to at most max_len characters and strip any trailing '-'.",
 src={"slug.py": '''"""URL slugs."""

def slugify(text):
    return text.lower()
'''},
 tests={"test_slug.py": '''from slug import slugify

def test_basic():
    assert slugify("Hello, World!") == "hello-world"
    assert slugify("  --A  b__c-- ") == "a-b-c"

def test_max_len():
    assert slugify("hello world foo", max_len=8) == "hello-wo"
    assert slugify("hello world", max_len=6) == "hello"
    assert slugify("abc", max_len=10) == "abc"
'''},
 solution={"slug.py": '''import re


def slugify(text, max_len=None):
    out = re.sub(r"[^a-z0-9]+", "-", text.lower()).strip("-")
    if max_len is not None:
        out = out[:max_len].rstrip("-")
    return out
'''})
