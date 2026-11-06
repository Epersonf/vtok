from cache import SimpleCache

def test_evicts_lru():
    c = SimpleCache(2)
    c.put("a", 1); c.put("b", 2); c.get("a"); c.put("c", 3)
    assert "b" not in c and "a" in c and "c" in c and len(c) == 2

def test_put_existing_refreshes():
    c = SimpleCache(2)
    c.put("a", 1); c.put("b", 2); c.put("a", 10); c.put("c", 3)
    assert c.get("a") == 10 and "b" not in c

def test_contains_is_not_a_use():
    c = SimpleCache(2)
    c.put("a", 1); c.put("b", 2)
    assert "a" in c
    c.put("c", 3)
    assert "a" not in c

def test_keys_order_and_default():
    c = SimpleCache(3)
    c.put("a", 1); c.put("b", 2); c.put("c", 3); c.get("a")
    assert c.keys() == ["b", "c", "a"] and c.get("zz", 7) == 7
