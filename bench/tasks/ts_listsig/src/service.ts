import type { User, Role } from './types.ts';

export class UserService {
  private users: User[] = [];

  add(user: User): void {
    this.users.push(user);
  }

  list(): User[] {
    return [...this.users];
  }
}
