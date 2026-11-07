"""Task `ts_emitter` (typescript, target emitter.ts): the instruction, the project to change, its tests and a reference solution."""
TASK = dict(lang="typescript", target="emitter.ts", cmd="node", instruction=
 "Class `Emitter` needs: `once(event, fn)` (listener runs at most once, then is removed), `off(event, fn)` returning true if a "
 "listener was removed, and `emit` must not break if a listener removes itself while emitting (iterate over a copy). "
 "Add `listenerCount(event): number`. Keep `on` and `emit` signatures; `emit` returns whether any listener ran.",
 src={"emitter.ts": '''type Listener = (...args: any[]) => void;

export class Emitter {
  private handlers: Map<string, Listener[]> = new Map();

  on(event: string, fn: Listener): void {
    const list = this.handlers.get(event) ?? [];
    list.push(fn);
    this.handlers.set(event, list);
  }

  emit(event: string, ...args: any[]): boolean {
    const list = this.handlers.get(event);
    if (!list || list.length === 0) {
      return false;
    }
    for (const fn of list) {
      fn(...args);
    }
    return true;
  }
}
'''},
 tests={"emitter.test.ts": '''import test from 'node:test';
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
'''},
 solution={"emitter.ts": '''type Listener = (...args: any[]) => void;

export class Emitter {
  private handlers: Map<string, Listener[]> = new Map();

  on(event: string, fn: Listener): void {
    const list = this.handlers.get(event) ?? [];
    list.push(fn);
    this.handlers.set(event, list);
  }

  once(event: string, fn: Listener): void {
    const wrapper: Listener = (...args) => {
      this.off(event, wrapper);
      fn(...args);
    };
    this.on(event, wrapper);
  }

  off(event: string, fn: Listener): boolean {
    const list = this.handlers.get(event);
    if (!list) {
      return false;
    }
    const i = list.indexOf(fn);
    if (i < 0) {
      return false;
    }
    list.splice(i, 1);
    return true;
  }

  listenerCount(event: string): number {
    return (this.handlers.get(event) ?? []).length;
  }

  emit(event: string, ...args: any[]): boolean {
    const list = this.handlers.get(event);
    if (!list || list.length === 0) {
      return false;
    }
    for (const fn of [...list]) {
      fn(...args);
    }
    return true;
  }
}
'''})
