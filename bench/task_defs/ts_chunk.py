"""Task `ts_chunk` (typescript, target arrays.ts): the instruction, the project to change, its tests and a reference solution."""
TASK = dict(lang="typescript", target="arrays.ts", cmd="node", instruction=
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
