"""Task `ts_matrix` (typescript, target matrix.ts): the instruction, the project to change, its tests and a reference solution."""
TASK = dict(lang="typescript", target="matrix.ts", cmd="node", instruction=
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
