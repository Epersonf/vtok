class AuthMgr:
    """Manages auth."""
    def validate(self, user: str, password: str) -> bool:
        if not user or not password:
            return False
        return db.query(user, password)

    def login(self, user, password):
        if self.validate(user, password):
            session = self.create_session(user)
            return session
        else:
            raise ValueError("bad credentials")
