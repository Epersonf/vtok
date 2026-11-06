export type Role = 'admin' | 'member' | 'guest';

export interface User {
  id: number;
  name: string;
  role: Role;
}
