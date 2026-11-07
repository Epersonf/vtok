"""Task `ts_group` (typescript, target group.ts): the instruction, the project to change, its tests and a reference solution."""
TASK = dict(lang="typescript", target="group.ts", cmd="node", instruction=
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
