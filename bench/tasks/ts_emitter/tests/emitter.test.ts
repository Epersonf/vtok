import test from 'node:test';
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
