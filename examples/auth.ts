// auth
export class AuthMgr {
  constructor(private db: Db) {}
  validate(user: string, password: string): boolean {
    if (!user || !password) {
      return false;
    }
    return this.db.query(user, password);
  }
  login(user: string, password: string) {
    if (this.validate(user, password)) {
      const session = this.createSession(user);
      return session;
    } else {
      throw new Error(`bad credentials for ${user}`);
    }
  }
}
