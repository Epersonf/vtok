"""Task `ts_listsig` (typescript, target service.ts): the instruction, the project to change, its tests and a reference solution."""
TASK = dict(lang="typescript", target="service.ts", cmd="node", context=["types.ts"], instruction=
 "In `UserService`, change `list` to `list(limit: number = 10, offset: number = 0): User[]` returning that window of users (insertion order), "
 "and add `findByRole(role: Role, limit?: number): User[]` returning users with that role (at most `limit` when given). Use the existing types.",
 src={"types.ts": '''export type Role = 'admin' | 'member' | 'guest';

export interface User {
  id: number;
  name: string;
  role: Role;
}
''', "service.ts": '''import type { User, Role } from './types.ts';

export class UserService {
  private users: User[] = [];

  add(user: User): void {
    this.users.push(user);
  }

  list(): User[] {
    return [...this.users];
  }
}
'''},
 tests={"service.test.ts": '''import test from 'node:test';
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
'''},
 solution={"service.ts": '''import type { User, Role } from './types.ts';

export class UserService {
  private users: User[] = [];

  add(user: User): void {
    this.users.push(user);
  }

  list(limit: number = 10, offset: number = 0): User[] {
    return this.users.slice(offset, offset + limit);
  }

  findByRole(role: Role, limit?: number): User[] {
    const r = this.users.filter(u => u.role === role);
    return limit === undefined ? r : r.slice(0, limit);
  }
}
'''})
