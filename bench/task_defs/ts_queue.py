"""Task `ts_queue` (typescript, target queue.ts): the instruction, the project to change, its tests and a reference solution."""
TASK = dict(lang="typescript", target="queue.ts", cmd="node", instruction=
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
