"""Project-level alias table: one stable table for many files, paid once (spec 4.4)."""
from collections import Counter
from .aliasing import plan_aliases

def plan_project(lang, sources, min_count=2):
    counts, excluded, avoid, reserved = Counter(), set(), set(), set()
    for src in sources:
        try:
            a = lang.analyze(src)
        except Exception:
            continue
        counts.update(a.counts); excluded |= a.excluded; avoid |= a.avoid; reserved |= a.reserved
    return plan_aliases(counts, avoid=avoid, reserved=reserved, excluded=excluded, min_count=min_count)
