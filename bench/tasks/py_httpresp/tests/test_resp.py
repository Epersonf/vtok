import pytest
from resp import build_response, MESSAGES

def test_plain():
    r = build_response(200)
    assert r == {"status": 200, "ok": True, "message": MESSAGES[200]}

def test_detail():
    assert build_response(404, "user 7")["message"] == MESSAGES[404] + ": user 7"

def test_retry_after():
    assert build_response(429, retry_after=30)["retry_after"] == 30
    assert "retry_after" not in build_response(429)
    assert "retry_after" not in build_response(500, retry_after=5)

def test_unknown():
    with pytest.raises(KeyError):
        build_response(418)
