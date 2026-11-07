"""Task `ts_retry` (typescript, target retry.ts): the instruction, the project to change, its tests and a reference solution."""
TASK = dict(lang="typescript", target="retry.ts", cmd="node", instruction=
 "`retry(fn, attempts)` should call the async `fn` up to `attempts` times total until it resolves, returning its value. If every attempt "
 "rejects, reject with the LAST error. Throw `new RangeError('attempts must be >= 1')` (synchronously rejected promise is fine) if attempts < 1. "
 "Add an optional third parameter `onRetry?: (error: unknown, attempt: number) => void` called after each failed attempt that will be retried "
 "(attempt is 1-based; not called after the final failure).",
 src={"retry.ts": '''// Retry helper

export async function retry<T>(fn: () => Promise<T>, attempts: number): Promise<T> {
  return fn();
}
'''},
 tests={"retry.test.ts": '''import test from 'node:test';
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
'''},
 solution={"retry.ts": '''export async function retry<T>(
  fn: () => Promise<T>,
  attempts: number,
  onRetry?: (error: unknown, attempt: number) => void,
): Promise<T> {
  if (attempts < 1) {
    throw new RangeError('attempts must be >= 1');
  }
  let lastError: unknown;
  for (let attempt = 1; attempt <= attempts; attempt++) {
    try {
      return await fn();
    } catch (e) {
      lastError = e;
      if (attempt < attempts && onRetry) {
        onRetry(e, attempt);
      }
    }
  }
  throw lastError;
}
'''})
