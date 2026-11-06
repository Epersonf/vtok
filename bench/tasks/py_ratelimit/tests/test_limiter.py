from limiter import RateLimiter

def test_basic():
    r = RateLimiter(2, 10)
    assert r.allow("a", 0) and r.allow("a", 1) and not r.allow("a", 2)
    assert r.allow("a", 11)

def test_reset():
    r = RateLimiter(1, 10)
    assert r.allow("a", 0) and not r.allow("a", 1)
    r.reset("a")
    assert r.allow("a", 2)

def test_override():
    r = RateLimiter(1, 10)
    r.set_limit("vip", 3)
    assert all(r.allow("vip", t) for t in (0, 1, 2)) and not r.allow("vip", 3)
    assert r.allow("x", 0) and not r.allow("x", 1)

def test_remaining():
    r = RateLimiter(3, 10)
    assert r.remaining("a", 0) == 3
    r.allow("a", 0); r.allow("a", 1)
    assert r.remaining("a", 2) == 1
    r.allow("a", 2)
    assert r.remaining("a", 3) == 0
    assert r.remaining("a", 20) == 3
