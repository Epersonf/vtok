import test from 'node:test';
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
