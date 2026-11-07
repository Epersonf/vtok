"""Task `py_intervals` (python, target intervals.py): the instruction, the project to change, its tests and a reference solution."""
TASK = dict(lang="python", target="intervals.py", cmd="py", instruction=
 "`merge_intervals(intervals)` takes a list of (start, end) tuples and must return the merged, sorted list of tuples. "
 "It is wrong for unsorted input and does not merge intervals that merely touch (e.g. (1,2),(2,3) -> (1,3)). Fix it. "
 "It must not mutate its input.",
 src={"intervals.py": '''"""Interval utilities."""

def merge_intervals(intervals):
    """Merge overlapping intervals."""
    merged = []
    for start, end in intervals:
        if merged and start < merged[-1][1]:
            merged[-1] = (merged[-1][0], max(merged[-1][1], end))
        else:
            merged.append((start, end))
    return merged
'''},
 tests={"test_intervals.py": '''from intervals import merge_intervals

def test_unsorted():
    assert merge_intervals([(5, 6), (1, 3), (2, 4)]) == [(1, 4), (5, 6)]

def test_touching():
    assert merge_intervals([(1, 2), (2, 3)]) == [(1, 3)]

def test_no_mutation():
    data = [(3, 4), (1, 2)]
    merge_intervals(data)
    assert data == [(3, 4), (1, 2)]

def test_empty():
    assert merge_intervals([]) == []
'''},
 solution={"intervals.py": '''def merge_intervals(intervals):
    merged = []
    for start, end in sorted(intervals):
        if merged and start <= merged[-1][1]:
            merged[-1] = (merged[-1][0], max(merged[-1][1], end))
        else:
            merged.append((start, end))
    return merged
'''})
