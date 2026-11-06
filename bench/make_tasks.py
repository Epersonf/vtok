"""Materialize the self-validation tasks: bench/tasks/<name>/{task.json,src,tests,solution}."""
import json, pathlib, shutil
ROOT = pathlib.Path(__file__).parent / "tasks"

T = {}

T["py_paginate"] = dict(lang="python", target="pager.py", cmd="py", instruction=
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

T["py_stack_peek"] = dict(lang="python", target="stack.py", cmd="py", instruction=
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

T["py_parse_kv"] = dict(lang="python", target="kv.py", cmd="py", instruction=
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

T["py_intervals"] = dict(lang="python", target="intervals.py", cmd="py", instruction=
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

T["py_ratelimit"] = dict(lang="python", target="limiter.py", cmd="py", instruction=
 "Class `RateLimiter` allows `limit` calls per key within a sliding window of `window` seconds. Add: "
 "(1) method `reset(key)` that forgets a key's history; (2) an optional per-key override: `set_limit(key, limit)` so that "
 "`allow(key, now)` uses that key's limit instead of the default; (3) method `remaining(key, now)` returning how many more "
 "calls are allowed right now for that key (never negative). `allow` must keep its current behaviour otherwise.",
 src={"limiter.py": '''"""Sliding-window rate limiter."""
from collections import defaultdict, deque


class RateLimiter:
    """Allow `limit` calls per `window` seconds, per key."""

    def __init__(self, limit, window):
        self.limit = limit
        self.window = window
        self._hits = defaultdict(deque)

    def _prune(self, key, now):
        hits = self._hits[key]
        while hits and hits[0] <= now - self.window:
            hits.popleft()
        return hits

    def allow(self, key, now):
        """Record a call if allowed; return True when allowed."""
        hits = self._prune(key, now)
        if len(hits) >= self.limit:
            return False
        hits.append(now)
        return True
'''},
 tests={"test_limiter.py": '''from limiter import RateLimiter

def test_basic():
    r = RateLimiter(2, 10)
    assert r.allow("a", 0) and r.allow("a", 1) and not r.allow("a", 2)
    assert r.allow("a", 11)

def test_reset():
    r = RateLimiter(1, 10)
    assert r.allow("a", 0) and not r.allow("a", 1)
    r.reset("a")
    assert r.allow("a", 2)

def test_override():
    r = RateLimiter(1, 10)
    r.set_limit("vip", 3)
    assert all(r.allow("vip", t) for t in (0, 1, 2)) and not r.allow("vip", 3)
    assert r.allow("x", 0) and not r.allow("x", 1)

def test_remaining():
    r = RateLimiter(3, 10)
    assert r.remaining("a", 0) == 3
    r.allow("a", 0); r.allow("a", 1)
    assert r.remaining("a", 2) == 1
    r.allow("a", 2)
    assert r.remaining("a", 3) == 0
    assert r.remaining("a", 20) == 3
'''},
 solution={"limiter.py": '''from collections import defaultdict, deque


class RateLimiter:
    def __init__(self, limit, window):
        self.limit = limit
        self.window = window
        self._hits = defaultdict(deque)
        self._limits = {}

    def _prune(self, key, now):
        hits = self._hits[key]
        while hits and hits[0] <= now - self.window:
            hits.popleft()
        return hits

    def _limit_for(self, key):
        return self._limits.get(key, self.limit)

    def set_limit(self, key, limit):
        self._limits[key] = limit

    def reset(self, key):
        self._hits.pop(key, None)

    def remaining(self, key, now):
        return max(0, self._limit_for(key) - len(self._prune(key, now)))

    def allow(self, key, now):
        hits = self._prune(key, now)
        if len(hits) >= self._limit_for(key):
            return False
        hits.append(now)
        return True
'''})

T["py_flatten"] = dict(lang="python", target="flat.py", cmd="py", instruction=
 "Add `flatten(d, sep='.')` to the module: flattens nested dicts into one dict with joined keys "
 "({'a': {'b': 1, 'c': {'d': 2}}, 'e': 3} -> {'a.b': 1, 'a.c.d': 2, 'e': 3}); non-dict values (including lists) are kept as-is; "
 "an empty nested dict is kept as an empty dict under its key. Also add `unflatten(d, sep='.')` as the inverse, and keep `get_path` unchanged.",
 src={"flat.py": '''"""Nested dict helpers."""

def get_path(d, path, default=None):
    """Read d['a']['b'] via 'a.b'."""
    cur = d
    for part in path.split("."):
        if not isinstance(cur, dict) or part not in cur:
            return default
        cur = cur[part]
    return cur
'''},
 tests={"test_flat.py": '''from flat import flatten, unflatten, get_path

def test_flatten():
    assert flatten({"a": {"b": 1, "c": {"d": 2}}, "e": 3}) == {"a.b": 1, "a.c.d": 2, "e": 3}

def test_sep_lists_empty():
    assert flatten({"a": {"b": [1, {"x": 1}]}, "z": {}}, sep="/") == {"a/b": [1, {"x": 1}], "z": {}}

def test_unflatten():
    assert unflatten({"a.b": 1, "a.c.d": 2, "e": 3}) == {"a": {"b": 1, "c": {"d": 2}}, "e": 3}

def test_roundtrip():
    d = {"a": {"b": {"c": 1}}, "k": 2}
    assert unflatten(flatten(d)) == d

def test_get_path():
    assert get_path({"a": {"b": 5}}, "a.b") == 5 and get_path({}, "a.b", 9) == 9
'''},
 solution={"flat.py": '''def get_path(d, path, default=None):
    cur = d
    for part in path.split("."):
        if not isinstance(cur, dict) or part not in cur:
            return default
        cur = cur[part]
    return cur


def flatten(d, sep="."):
    out = {}
    for key, value in d.items():
        if isinstance(value, dict) and value:
            for k, v in flatten(value, sep).items():
                out[key + sep + k] = v
        else:
            out[key] = value
    return out


def unflatten(d, sep="."):
    out = {}
    for key, value in d.items():
        cur = out
        parts = key.split(sep)
        for part in parts[:-1]:
            cur = cur.setdefault(part, {})
        cur[parts[-1]] = value
    return out
'''})

T["ts_chunk"] = dict(lang="typescript", target="arrays.ts", cmd="node", instruction=
 "`chunk(arr, size)` must split an array into consecutive chunks of `size` (last one may be shorter) and throw "
 "`new RangeError('size must be positive')` when size < 1 or not an integer. It currently loops forever for some inputs. "
 "Also add and export `unique<T>(arr: T[]): T[]` (first occurrence kept, order preserved) and `zip<A, B>(a: A[], b: B[]): [A, B][]` "
 "(length of the shorter).",
 src={"arrays.ts": '''// Array helpers

export function chunk<T>(arr: T[], size: number): T[][] {
  const out: T[][] = [];
  let i = 0;
  while (i < arr.length) {
    out.push(arr.slice(i, i + size));
  }
  return out;
}
'''},
 tests={"arrays.test.ts": '''import test from 'node:test';
import assert from 'node:assert/strict';
import { chunk, unique, zip } from './arrays.ts';

test('chunk', () => {
  assert.deepEqual(chunk([1, 2, 3, 4, 5], 2), [[1, 2], [3, 4], [5]]);
  assert.deepEqual(chunk([], 3), []);
});
test('chunk invalid', () => {
  assert.throws(() => chunk([1], 0), { name: 'RangeError', message: 'size must be positive' });
  assert.throws(() => chunk([1], 1.5), RangeError);
});
test('unique', () => {
  assert.deepEqual(unique([3, 1, 3, 2, 1]), [3, 1, 2]);
});
test('zip', () => {
  assert.deepEqual(zip([1, 2, 3], ['a', 'b']), [[1, 'a'], [2, 'b']]);
});
'''},
 solution={"arrays.ts": '''export function chunk<T>(arr: T[], size: number): T[][] {
  if (!Number.isInteger(size) || size < 1) {
    throw new RangeError('size must be positive');
  }
  const out: T[][] = [];
  for (let i = 0; i < arr.length; i += size) {
    out.push(arr.slice(i, i + size));
  }
  return out;
}

export function unique<T>(arr: T[]): T[] {
  return [...new Set(arr)];
}

export function zip<A, B>(a: A[], b: B[]): [A, B][] {
  const n = Math.min(a.length, b.length);
  const out: [A, B][] = [];
  for (let i = 0; i < n; i++) {
    out.push([a[i], b[i]]);
  }
  return out;
}
'''})

T["ts_emitter"] = dict(lang="typescript", target="emitter.ts", cmd="node", instruction=
 "Class `Emitter` needs: `once(event, fn)` (listener runs at most once, then is removed), `off(event, fn)` returning true if a "
 "listener was removed, and `emit` must not break if a listener removes itself while emitting (iterate over a copy). "
 "Add `listenerCount(event): number`. Keep `on` and `emit` signatures; `emit` returns whether any listener ran.",
 src={"emitter.ts": '''type Listener = (...args: any[]) => void;

export class Emitter {
  private handlers: Map<string, Listener[]> = new Map();

  on(event: string, fn: Listener): void {
    const list = this.handlers.get(event) ?? [];
    list.push(fn);
    this.handlers.set(event, list);
  }

  emit(event: string, ...args: any[]): boolean {
    const list = this.handlers.get(event);
    if (!list || list.length === 0) {
      return false;
    }
    for (const fn of list) {
      fn(...args);
    }
    return true;
  }
}
'''},
 tests={"emitter.test.ts": '''import test from 'node:test';
import assert from 'node:assert/strict';
import { Emitter } from './emitter.ts';

test('on/emit', () => {
  const e = new Emitter(); const got: number[] = [];
  e.on('x', (n: number) => got.push(n));
  assert.equal(e.emit('x', 1), true);
  assert.equal(e.emit('y'), false);
  assert.deepEqual(got, [1]);
});
test('once', () => {
  const e = new Emitter(); let n = 0;
  e.once('x', () => { n++; });
  e.emit('x'); e.emit('x');
  assert.equal(n, 1);
  assert.equal(e.listenerCount('x'), 0);
});
test('off', () => {
  const e = new Emitter(); const f = () => {};
  e.on('x', f);
  assert.equal(e.off('x', f), true);
  assert.equal(e.off('x', f), false);
});
test('self removal while emitting', () => {
  const e = new Emitter(); const seen: string[] = [];
  const a = () => { seen.push('a'); e.off('x', a); };
  e.on('x', a); e.on('x', () => seen.push('b'));
  e.emit('x');
  assert.deepEqual(seen, ['a', 'b']);
  assert.equal(e.listenerCount('x'), 1);
});
'''},
 solution={"emitter.ts": '''type Listener = (...args: any[]) => void;

export class Emitter {
  private handlers: Map<string, Listener[]> = new Map();

  on(event: string, fn: Listener): void {
    const list = this.handlers.get(event) ?? [];
    list.push(fn);
    this.handlers.set(event, list);
  }

  once(event: string, fn: Listener): void {
    const wrapper: Listener = (...args) => {
      this.off(event, wrapper);
      fn(...args);
    };
    this.on(event, wrapper);
  }

  off(event: string, fn: Listener): boolean {
    const list = this.handlers.get(event);
    if (!list) {
      return false;
    }
    const i = list.indexOf(fn);
    if (i < 0) {
      return false;
    }
    list.splice(i, 1);
    return true;
  }

  listenerCount(event: string): number {
    return (this.handlers.get(event) ?? []).length;
  }

  emit(event: string, ...args: any[]): boolean {
    const list = this.handlers.get(event);
    if (!list || list.length === 0) {
      return false;
    }
    for (const fn of [...list]) {
      fn(...args);
    }
    return true;
  }
}
'''})

T["py_lru"] = dict(lang="python", target="cache.py", cmd="py", instruction=
 "`SimpleCache(capacity)` currently never evicts. Make it a least-recently-used cache: `get(key, default=None)` and `put(key, value)` "
 "both count as a use; when `put` would exceed `capacity`, evict the least recently used key. Add `__contains__` (does NOT count as a use), "
 "`__len__`, and `keys()` returning keys from least to most recently used.",
 src={"cache.py": '''"""Tiny cache."""

class SimpleCache:
    """Key/value store with a nominal capacity."""

    def __init__(self, capacity):
        self.capacity = capacity
        self._data = {}

    def get(self, key, default=None):
        return self._data.get(key, default)

    def put(self, key, value):
        self._data[key] = value
'''},
 tests={"test_cache.py": '''from cache import SimpleCache

def test_evicts_lru():
    c = SimpleCache(2)
    c.put("a", 1); c.put("b", 2); c.get("a"); c.put("c", 3)
    assert "b" not in c and "a" in c and "c" in c and len(c) == 2

def test_put_existing_refreshes():
    c = SimpleCache(2)
    c.put("a", 1); c.put("b", 2); c.put("a", 10); c.put("c", 3)
    assert c.get("a") == 10 and "b" not in c

def test_contains_is_not_a_use():
    c = SimpleCache(2)
    c.put("a", 1); c.put("b", 2)
    assert "a" in c
    c.put("c", 3)
    assert "a" not in c

def test_keys_order_and_default():
    c = SimpleCache(3)
    c.put("a", 1); c.put("b", 2); c.put("c", 3); c.get("a")
    assert c.keys() == ["b", "c", "a"] and c.get("zz", 7) == 7
'''},
 solution={"cache.py": '''from collections import OrderedDict


class SimpleCache:
    def __init__(self, capacity):
        self.capacity = capacity
        self._data = OrderedDict()

    def get(self, key, default=None):
        if key in self._data:
            self._data.move_to_end(key)
            return self._data[key]
        return default

    def put(self, key, value):
        self._data[key] = value
        self._data.move_to_end(key)
        while len(self._data) > self.capacity:
            self._data.popitem(last=False)

    def __contains__(self, key):
        return key in self._data

    def __len__(self):
        return len(self._data)

    def keys(self):
        return list(self._data)
'''})

T["py_wordcount"] = dict(lang="python", target="tally.py", cmd="py", instruction=
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

T["ts_retry"] = dict(lang="typescript", target="retry.ts", cmd="node", instruction=
 "`retry(fn, attempts)` should call the async `fn` up to `attempts` times total until it resolves, returning its value. If every attempt "
 "rejects, reject with the LAST error. Throw `new RangeError('attempts must be >= 1')` (synchronously rejected promise is fine) if attempts < 1. "
 "Add an optional third parameter `onRetry?: (error: unknown, attempt: number) => void` called after each failed attempt that will be retried "
 "(attempt is 1-based; not called after the final failure).",
 src={"retry.ts": '''// Retry helper

export async function retry<T>(fn: () => Promise<T>, attempts: number): Promise<T> {
  return fn();
}
'''},
 tests={"retry.test.ts": '''import test from 'node:test';
import assert from 'node:assert/strict';
import { retry } from './retry.ts';

test('succeeds after failures', async () => {
  let n = 0;
  const v = await retry(async () => { n++; if (n < 3) throw new Error('e' + n); return 'ok'; }, 5);
  assert.equal(v, 'ok'); assert.equal(n, 3);
});
test('rejects with last error', async () => {
  let n = 0;
  await assert.rejects(retry(async () => { n++; throw new Error('e' + n); }, 3), { message: 'e3' });
  assert.equal(n, 3);
});
test('onRetry calls', async () => {
  const seen: number[] = [];
  await assert.rejects(retry(async () => { throw new Error('x'); }, 3, (_e, a) => seen.push(a)));
  assert.deepEqual(seen, [1, 2]);
});
test('invalid attempts', async () => {
  await assert.rejects(retry(async () => 1, 0), RangeError);
});
'''},
 solution={"retry.ts": '''export async function retry<T>(
  fn: () => Promise<T>,
  attempts: number,
  onRetry?: (error: unknown, attempt: number) => void,
): Promise<T> {
  if (attempts < 1) {
    throw new RangeError('attempts must be >= 1');
  }
  let lastError: unknown;
  for (let attempt = 1; attempt <= attempts; attempt++) {
    try {
      return await fn();
    } catch (e) {
      lastError = e;
      if (attempt < attempts && onRetry) {
        onRetry(e, attempt);
      }
    }
  }
  throw lastError;
}
'''})

T["ts_matrix"] = dict(lang="typescript", target="matrix.ts", cmd="node", instruction=
 "Add and export `transpose(m: number[][]): number[][]` (empty matrix -> []) and `multiply(a: number[][], b: number[][]): number[][]` "
 "(standard matrix product; throw `new Error('dimension mismatch')` when a's column count != b's row count). "
 "Also make `identity(n)` throw `new RangeError('n must be positive')` for n < 1. Keep `identity` otherwise unchanged.",
 src={"matrix.ts": '''// Small matrix helpers

export function identity(n: number): number[][] {
  const out: number[][] = [];
  for (let i = 0; i < n; i++) {
    const row: number[] = [];
    for (let j = 0; j < n; j++) {
      row.push(i === j ? 1 : 0);
    }
    out.push(row);
  }
  return out;
}
'''},
 tests={"matrix.test.ts": '''import test from 'node:test';
import assert from 'node:assert/strict';
import { identity, transpose, multiply } from './matrix.ts';

test('transpose', () => {
  assert.deepEqual(transpose([[1, 2, 3], [4, 5, 6]]), [[1, 4], [2, 5], [3, 6]]);
  assert.deepEqual(transpose([]), []);
});
test('multiply', () => {
  assert.deepEqual(multiply([[1, 2], [3, 4]], [[5, 6], [7, 8]]), [[19, 22], [43, 50]]);
  assert.deepEqual(multiply([[1, 2, 3]], [[1], [1], [1]]), [[6]]);
  assert.deepEqual(multiply(identity(2), [[3, 4], [5, 6]]), [[3, 4], [5, 6]]);
});
test('mismatch', () => {
  assert.throws(() => multiply([[1, 2]], [[1, 2]]), { message: 'dimension mismatch' });
});
test('identity validation', () => {
  assert.throws(() => identity(0), RangeError);
  assert.deepEqual(identity(1), [[1]]);
});
'''},
 solution={"matrix.ts": '''export function identity(n: number): number[][] {
  if (n < 1) {
    throw new RangeError('n must be positive');
  }
  const out: number[][] = [];
  for (let i = 0; i < n; i++) {
    const row: number[] = [];
    for (let j = 0; j < n; j++) {
      row.push(i === j ? 1 : 0);
    }
    out.push(row);
  }
  return out;
}

export function transpose(m: number[][]): number[][] {
  if (m.length === 0) {
    return [];
  }
  return m[0].map((_, j) => m.map((row) => row[j]));
}

export function multiply(a: number[][], b: number[][]): number[][] {
  const cols = a.length ? a[0].length : 0;
  if (cols !== b.length) {
    throw new Error('dimension mismatch');
  }
  const width = b.length ? b[0].length : 0;
  const out: number[][] = [];
  for (let i = 0; i < a.length; i++) {
    const row: number[] = [];
    for (let j = 0; j < width; j++) {
      let sum = 0;
      for (let k = 0; k < cols; k++) {
        sum += a[i][k] * b[k][j];
      }
      row.push(sum);
    }
    out.push(row);
  }
  return out;
}
'''})

T["py_slug"] = dict(lang="python", target="slug.py", cmd="py", instruction=
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

T["py_account"] = dict(lang="python", target="account.py", cmd="py", instruction=
 "Add to `Account`: class `InsufficientFunds(Exception)`; `withdraw` must raise it (instead of going negative) when amount > balance, "
 "and raise ValueError for amount <= 0 (same for `deposit`); a `history` list of ('deposit'|'withdraw', amount) tuples recorded for "
 "successful operations; and `transfer(other, amount)` that withdraws from self and deposits to other (nothing changes on failure).",
 src={"account.py": '''"""Bank account."""

class Account:
    def __init__(self, balance=0):
        self.balance = balance

    def deposit(self, amount):
        self.balance += amount

    def withdraw(self, amount):
        self.balance -= amount
'''},
 tests={"test_account.py": '''import pytest
from account import Account, InsufficientFunds

def test_ops_and_history():
    a = Account(10); a.deposit(5); a.withdraw(3)
    assert a.balance == 12 and a.history == [("deposit", 5), ("withdraw", 3)]

def test_errors_leave_state():
    a = Account(5)
    with pytest.raises(InsufficientFunds):
        a.withdraw(6)
    for f in (a.deposit, a.withdraw):
        with pytest.raises(ValueError):
            f(0)
    assert a.balance == 5 and a.history == []

def test_transfer():
    a, b = Account(10), Account(1)
    a.transfer(b, 4)
    assert (a.balance, b.balance) == (6, 5)
    with pytest.raises(InsufficientFunds):
        a.transfer(b, 100)
    assert (a.balance, b.balance) == (6, 5)
'''},
 solution={"account.py": '''class InsufficientFunds(Exception):
    pass


class Account:
    def __init__(self, balance=0):
        self.balance = balance
        self.history = []

    def deposit(self, amount):
        if amount <= 0:
            raise ValueError("amount must be positive")
        self.balance += amount
        self.history.append(("deposit", amount))

    def withdraw(self, amount):
        if amount <= 0:
            raise ValueError("amount must be positive")
        if amount > self.balance:
            raise InsufficientFunds()
        self.balance -= amount
        self.history.append(("withdraw", amount))

    def transfer(self, other, amount):
        self.withdraw(amount)
        other.deposit(amount)
'''})

T["ts_group"] = dict(lang="typescript", target="group.ts", cmd="node", instruction=
 "Add and export: `groupBy<T>(arr: T[], key: (x: T) => string): Record<string, T[]>` (keeps input order inside each group), "
 "`countBy<T>(arr: T[], key: (x: T) => string): Record<string, number>`, and `partition<T>(arr: T[], pred: (x: T) => boolean): [T[], T[]]` "
 "(matching first). Keep `sum` unchanged.",
 src={"group.ts": '''// Collection helpers

export function sum(xs: number[]): number {
  let total = 0;
  for (const x of xs) {
    total += x;
  }
  return total;
}
'''},
 tests={"group.test.ts": '''import test from 'node:test';
import assert from 'node:assert/strict';
import { sum, groupBy, countBy, partition } from './group.ts';

test('groupBy/countBy', () => {
  const w = ['apple', 'avocado', 'banana', 'blueberry', 'cherry'];
  assert.deepEqual(groupBy(w, (s) => s[0]), { a: ['apple', 'avocado'], b: ['banana', 'blueberry'], c: ['cherry'] });
  assert.deepEqual(countBy(w, (s) => s[0]), { a: 2, b: 2, c: 1 });
});
test('partition', () => {
  assert.deepEqual(partition([1, 2, 3, 4, 5], (n) => n % 2 === 0), [[2, 4], [1, 3, 5]]);
});
test('sum', () => { assert.equal(sum([1, 2, 3]), 6); });
'''},
 solution={"group.ts": '''export function sum(xs: number[]): number {
  let total = 0;
  for (const x of xs) {
    total += x;
  }
  return total;
}

export function groupBy<T>(arr: T[], key: (x: T) => string): Record<string, T[]> {
  const out: Record<string, T[]> = {};
  for (const x of arr) {
    const k = key(x);
    (out[k] ??= []).push(x);
  }
  return out;
}

export function countBy<T>(arr: T[], key: (x: T) => string): Record<string, number> {
  const out: Record<string, number> = {};
  for (const x of arr) {
    const k = key(x);
    out[k] = (out[k] ?? 0) + 1;
  }
  return out;
}

export function partition<T>(arr: T[], pred: (x: T) => boolean): [T[], T[]] {
  const yes: T[] = [];
  const no: T[] = [];
  for (const x of arr) {
    (pred(x) ? yes : no).push(x);
  }
  return [yes, no];
}
'''})

T["ts_queue"] = dict(lang="typescript", target="queue.ts", cmd="node", instruction=
 "Class `Queue<T>` only has `enqueue`. Add: `dequeue(): T | undefined` (FIFO, undefined when empty), `peek(): T | undefined`, a getter `size`, "
 "`isEmpty(): boolean`, and make the queue iterable (`[Symbol.iterator]`) yielding items front to back without consuming them.",
 src={"queue.ts": '''export class Queue<T> {
  private items: T[] = [];

  enqueue(item: T): void {
    this.items.push(item);
  }
}
'''},
 tests={"queue.test.ts": '''import test from 'node:test';
import assert from 'node:assert/strict';
import { Queue } from './queue.ts';

test('fifo', () => {
  const q = new Queue<number>(); q.enqueue(1); q.enqueue(2); q.enqueue(3);
  assert.equal(q.size, 3); assert.equal(q.peek(), 1); assert.equal(q.dequeue(), 1); assert.equal(q.size, 2);
  assert.deepEqual([...q], [2, 3]); assert.equal(q.size, 2);
});
test('empty', () => {
  const q = new Queue<string>();
  assert.equal(q.isEmpty(), true); assert.equal(q.dequeue(), undefined); assert.equal(q.peek(), undefined);
  q.enqueue('a'); assert.equal(q.isEmpty(), false);
});
'''},
 solution={"queue.ts": '''export class Queue<T> {
  private items: T[] = [];

  enqueue(item: T): void {
    this.items.push(item);
  }

  dequeue(): T | undefined {
    return this.items.shift();
  }

  peek(): T | undefined {
    return this.items[0];
  }

  get size(): number {
    return this.items.length;
  }

  isEmpty(): boolean {
    return this.items.length === 0;
  }

  *[Symbol.iterator](): Iterator<T> {
    yield* this.items;
  }
}
'''})

T["py_httpresp"] = dict(lang="python", target="resp.py", cmd="py", hide=5, instruction=
 "`build_response(code, detail=None)` must (1) append `': ' + detail` to the message when `detail` is given, "
 "(2) accept a keyword `retry_after=None` and, only when code is 429 and retry_after is not None, include the key 'retry_after' in the result, "
 "(3) raise KeyError for unknown codes (it currently falls back silently). Keep the existing messages unchanged.",
 src={"resp.py": '''MESSAGES = {
    200: "The request completed successfully and the resource is returned in the body",
    404: "The requested resource could not be found on this server",
    429: "Too many requests were received from this client in a short period of time",
    500: "The server encountered an unexpected condition and could not finish the request",
}


def build_response(code, detail=None):
    message = MESSAGES.get(code, "Unknown status")
    return {"status": code, "ok": code < 400, "message": message}
'''},
 tests={"test_resp.py": '''import pytest
from resp import build_response, MESSAGES

def test_plain():
    r = build_response(200)
    assert r == {"status": 200, "ok": True, "message": MESSAGES[200]}

def test_detail():
    assert build_response(404, "user 7")["message"] == MESSAGES[404] + ": user 7"

def test_retry_after():
    assert build_response(429, retry_after=30)["retry_after"] == 30
    assert "retry_after" not in build_response(429)
    assert "retry_after" not in build_response(500, retry_after=5)

def test_unknown():
    with pytest.raises(KeyError):
        build_response(418)
'''},
 solution={"resp.py": '''MESSAGES = {
    200: "The request completed successfully and the resource is returned in the body",
    404: "The requested resource could not be found on this server",
    429: "Too many requests were received from this client in a short period of time",
    500: "The server encountered an unexpected condition and could not finish the request",
}


def build_response(code, detail=None, retry_after=None):
    message = MESSAGES[code]
    if detail:
        message = message + ": " + detail
    r = {"status": code, "ok": code < 400, "message": message}
    if code == 429 and retry_after is not None:
        r["retry_after"] = retry_after
    return r
'''})

T["py_guard"] = dict(lang="python", target="orders.py", cmd="py", instruction=
 "Refactor `ship_cost(order)` to use guard clauses / early returns (no `else` branches, at most one level of nesting) "
 "and fix this bug: an order with a negative or zero quantity must raise ValueError (right now it returns 0). Behaviour must otherwise stay identical.",
 src={"orders.py": '''def ship_cost(order):
    if order is not None:
        if order["qty"] > 0:
            if order["country"] == "US":
                if order["weight"] > 10:
                    cost = 20
                else:
                    cost = 10
            else:
                if order["weight"] > 10:
                    cost = 45
                else:
                    cost = 30
            if order.get("express"):
                cost = cost * 2
            return cost
        else:
            return 0
    else:
        raise ValueError("no order")
'''},
 tests={"test_orders.py": '''import ast, inspect, pytest
import orders
from orders import ship_cost

def o(**k):
    d = {"qty": 1, "country": "US", "weight": 5}; d.update(k); return d

def test_costs():
    assert ship_cost(o()) == 10
    assert ship_cost(o(weight=11)) == 20
    assert ship_cost(o(country="FR")) == 30
    assert ship_cost(o(country="FR", weight=11)) == 45
    assert ship_cost(o(express=True)) == 20

def test_errors():
    with pytest.raises(ValueError): ship_cost(None)
    with pytest.raises(ValueError): ship_cost(o(qty=0))
    with pytest.raises(ValueError): ship_cost(o(qty=-2))

def test_no_else():
    tree = ast.parse(inspect.getsource(orders))
    assert not any(isinstance(n, ast.If) and n.orelse for n in ast.walk(tree))
'''},
 solution={"orders.py": '''def ship_cost(order):
    if order is None:
        raise ValueError("no order")
    if order["qty"] <= 0:
        raise ValueError("bad qty")
    heavy = order["weight"] > 10
    cost = (20 if heavy else 10) if order["country"] == "US" else (45 if heavy else 30)
    if order.get("express"):
        cost = cost * 2
    return cost
'''})

T["py_money"] = dict(lang="python", target="money.py", cmd="py", instruction=
 "Add to class `Money`: a static method `parse(text)` that turns strings like '12.50 USD' or '3 EUR' into a Money (cents as int, currency upper-case; raise ValueError on malformed text), "
 "and `__add__` returning a new Money (ValueError if currencies differ). Do not change existing members.",
 src={"money.py": '''class Money:
    """Amount in minor units."""

    def __init__(self, cents, currency):
        self.cents = cents
        self.currency = currency

    def __eq__(self, other):
        return self.cents == other.cents and self.currency == other.currency

    def __repr__(self):
        return f"Money({self.cents}, {self.currency!r})"

    def format(self):
        return f"{self.cents // 100}.{self.cents % 100:02d} {self.currency}"


def zero(currency):
    return Money(0, currency)
'''},
 tests={"test_money.py": '''import pytest
from money import Money, zero

def test_parse():
    assert Money.parse("12.50 USD") == Money(1250, "USD")
    assert Money.parse("3 eur") == Money(300, "EUR")
    assert Money.parse("0.05 USD") == Money(5, "USD")

def test_parse_bad():
    for s in ("", "abc", "12.5.1 USD", "12 US", "USD 12"):
        with pytest.raises(ValueError): Money.parse(s)

def test_add():
    assert Money(100, "USD") + Money(250, "USD") == Money(350, "USD")
    assert zero("USD") + Money(1, "USD") == Money(1, "USD")
    with pytest.raises(ValueError): Money(1, "USD") + Money(1, "EUR")

def test_unchanged():
    assert Money(1250, "USD").format() == "12.50 USD"
'''},
 solution={"money.py": '''import re


class Money:
    """Amount in minor units."""

    def __init__(self, cents, currency):
        self.cents = cents
        self.currency = currency

    def __eq__(self, other):
        return self.cents == other.cents and self.currency == other.currency

    def __repr__(self):
        return f"Money({self.cents}, {self.currency!r})"

    def format(self):
        return f"{self.cents // 100}.{self.cents % 100:02d} {self.currency}"

    @staticmethod
    def parse(text):
        m = re.fullmatch(r"(\\d+)(?:\\.(\\d{1,2}))? ([A-Za-z]{3})", text)
        if not m:
            raise ValueError("bad money")
        frac = (m.group(2) or "").ljust(2, "0")
        return Money(int(m.group(1)) * 100 + int(frac or 0), m.group(3).upper())

    def __add__(self, other):
        if self.currency != other.currency:
            raise ValueError("currency mismatch")
        return Money(self.cents + other.cents, self.currency)


def zero(currency):
    return Money(0, currency)
'''})

T["ts_listsig"] = dict(lang="typescript", target="service.ts", cmd="node", context=["types.ts"], instruction=
 "In `UserService`, change `list` to `list(limit: number = 10, offset: number = 0): User[]` returning that window of users (insertion order), "
 "and add `findByRole(role: Role, limit?: number): User[]` returning users with that role (at most `limit` when given). Use the existing types.",
 src={"types.ts": '''export type Role = 'admin' | 'member' | 'guest';

export interface User {
  id: number;
  name: string;
  role: Role;
}
''', "service.ts": '''import type { User, Role } from './types.ts';

export class UserService {
  private users: User[] = [];

  add(user: User): void {
    this.users.push(user);
  }

  list(): User[] {
    return [...this.users];
  }
}
'''},
 tests={"service.test.ts": '''import test from 'node:test';
import assert from 'node:assert/strict';
import { UserService } from './service.ts';

function make() {
  const s = new UserService();
  const roles = ['admin', 'member', 'guest'] as const;
  for (let i = 1; i <= 25; i++) s.add({ id: i, name: 'u' + i, role: roles[i % 3] });
  return s;
}
test('list window', () => {
  const s = make();
  assert.equal(s.list().length, 10);
  assert.deepEqual(s.list(3, 5).map(u => u.id), [6, 7, 8]);
  assert.equal(s.list(10, 20).length, 5);
});
test('by role', () => {
  const s = make();
  assert.ok(s.findByRole('admin').every(u => u.role === 'admin'));
  assert.equal(s.findByRole('guest').length, 8);
  assert.equal(s.findByRole('guest', 2).length, 2);
});
'''},
 solution={"service.ts": '''import type { User, Role } from './types.ts';

export class UserService {
  private users: User[] = [];

  add(user: User): void {
    this.users.push(user);
  }

  list(limit: number = 10, offset: number = 0): User[] {
    return this.users.slice(offset, offset + limit);
  }

  findByRole(role: Role, limit?: number): User[] {
    const r = this.users.filter(u => u.role === role);
    return limit === undefined ? r : r.slice(0, limit);
  }
}
'''})

def main():
    if ROOT.exists(): shutil.rmtree(ROOT)
    for name, t in T.items():
        d = ROOT / name
        for sub, files in (("src", t["src"]), ("tests", t["tests"]), ("solution", t["solution"])):
            (d / sub).mkdir(parents=True)
            for fn, body in files.items():
                (d / sub / fn).write_text(body)
        (d / "task.json").write_text(json.dumps({k: t[k] for k in ("lang", "target", "cmd", "instruction", "hide", "context") if k in t}, indent=1))
    print("tasks:", ", ".join(T))

if __name__ == "__main__":
    main()
