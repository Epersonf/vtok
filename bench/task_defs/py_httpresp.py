"""Task `py_httpresp` (python, target resp.py): the instruction, the project to change, its tests and a reference solution."""
TASK = dict(lang="python", target="resp.py", cmd="py", hide=5, instruction=
 "`build_response(code, detail=None)` must (1) append `': ' + detail` to the message when `detail` is given, "
 "(2) accept a keyword `retry_after=None` and, only when code is 429 and retry_after is not None, include the key 'retry_after' in the result, "
 "(3) raise KeyError for unknown codes (it currently falls back silently). Keep the existing messages unchanged.",
 src={"resp.py": '''MESSAGES = {
    200: "The request completed successfully and the resource is returned in the body",
    404: "The requested resource could not be found on this server",
    429: "Too many requests were received from this client in a short period of time",
    500: "The server encountered an unexpected condition and could not finish the request",
}


def build_response(code, detail=None):
    message = MESSAGES.get(code, "Unknown status")
    return {"status": code, "ok": code < 400, "message": message}
'''},
 tests={"test_resp.py": '''import pytest
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
'''},
 solution={"resp.py": '''MESSAGES = {
    200: "The request completed successfully and the resource is returned in the body",
    404: "The requested resource could not be found on this server",
    429: "Too many requests were received from this client in a short period of time",
    500: "The server encountered an unexpected condition and could not finish the request",
}


def build_response(code, detail=None, retry_after=None):
    message = MESSAGES[code]
    if detail:
        message = message + ": " + detail
    r = {"status": code, "ok": code < 400, "message": message}
    if code == 429 and retry_after is not None:
        r["retry_after"] = retry_after
    return r
'''})
