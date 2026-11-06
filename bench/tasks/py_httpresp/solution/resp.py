MESSAGES = {
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
