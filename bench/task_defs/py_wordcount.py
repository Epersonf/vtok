"""Task `py_wordcount` (python, target tally.py): the instruction, the project to change, its tests and a reference solution."""
TASK = dict(lang="python", target="tally.py", cmd="py", instruction=
 "`word_counts(text)` counts words case-sensitively and keeps punctuation attached. Fix it: lowercase everything and strip leading/trailing "
 "punctuation (characters in string.punctuation) from each word; drop words that become empty. Then add `top_n(text, n)` returning a list of "
 "(word, count) tuples sorted by count descending, ties broken alphabetically, at most n items (n <= 0 gives []).",
 src={"tally.py": '''"""Word statistics."""

def word_counts(text):
    """Count words in text."""
    counts = {}
    for word in text.split():
        counts[word] = counts.get(word, 0) + 1
    return counts
'''},
 tests={"test_tally.py": '''from tally import word_counts, top_n

def test_normalizes():
    assert word_counts("The cat, the CAT! (dog)") == {"the": 2, "cat": 2, "dog": 1}

def test_empty_words_dropped():
    assert word_counts("-- ... a") == {"a": 1}

def test_top_n():
    t = "b a b a c c c"
    assert top_n(t, 2) == [("c", 3), ("a", 2)]
    assert top_n(t, 10) == [("c", 3), ("a", 2), ("b", 2)]
    assert top_n(t, 0) == [] and top_n(t, -1) == []
'''},
 solution={"tally.py": '''import string


def word_counts(text):
    counts = {}
    for word in text.split():
        word = word.lower().strip(string.punctuation)
        if not word:
            continue
        counts[word] = counts.get(word, 0) + 1
    return counts


def top_n(text, n):
    if n <= 0:
        return []
    items = sorted(word_counts(text).items(), key=lambda kv: (-kv[1], kv[0]))
    return items[:n]
'''})
