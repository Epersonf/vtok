import test from 'node:test';
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
