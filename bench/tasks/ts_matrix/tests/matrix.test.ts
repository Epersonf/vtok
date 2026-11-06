import test from 'node:test';
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
