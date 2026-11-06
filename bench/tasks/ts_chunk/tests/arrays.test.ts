import test from 'node:test';
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
