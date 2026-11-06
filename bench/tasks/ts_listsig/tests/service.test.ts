import test from 'node:test';
import assert from 'node:assert/strict';
import { UserService } from './service.ts';

function make() {
  const s = new UserService();
  const roles = ['admin', 'member', 'guest'] as const;
  for (let i = 1; i <= 25; i++) s.add({ id: i, name: 'u' + i, role: roles[i % 3] });
  return s;
}
test('list window', () => {
  const s = make();
  assert.equal(s.list().length, 10);
  assert.deepEqual(s.list(3, 5).map(u => u.id), [6, 7, 8]);
  assert.equal(s.list(10, 20).length, 5);
});
test('by role', () => {
  const s = make();
  assert.ok(s.findByRole('admin').every(u => u.role === 'admin'));
  assert.equal(s.findByRole('guest').length, 8);
  assert.equal(s.findByRole('guest', 2).length, 2);
});
