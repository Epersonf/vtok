import test from 'node:test';
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
