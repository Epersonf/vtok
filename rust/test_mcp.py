"""End-to-end test of `vtok mcp` over stdio. usage: python rust/test_mcp.py [path/to/vtok]"""
import json, pathlib, re, subprocess, sys, tempfile

rbin = sys.argv[1] if len(sys.argv) > 1 else str(pathlib.Path(__file__).parent / "target/release/vtok")
p = subprocess.Popen([rbin, "mcp"], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
n = 0
def rpc(method, params=None, notify=False):
    global n
    msg = {"jsonrpc": "2.0", "method": method}
    if params is not None: msg["params"] = params
    if not notify: n += 1; msg["id"] = n
    p.stdin.write(json.dumps(msg) + "\n"); p.stdin.flush()
    return None if notify else json.loads(p.stdout.readline())
def tool(name, **args):
    r = rpc("tools/call", {"name": name, "arguments": args})["result"]
    return r["content"][0]["text"], bool(r.get("isError"))

init = rpc("initialize", {"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "t", "version": "0"}})["result"]
assert init["serverInfo"]["name"] == "vtok" and "vtok_read" in init["instructions"] or "Vtok" in init["instructions"]
rpc("notifications/initialized", notify=True)
names = [t["name"] for t in rpc("tools/list")["result"]["tools"]]
assert names == ["vtok_read", "vtok_chunk", "vtok_edit", "vtok_stats"], names

src = "".join(f"def f{i}(items, page):\n    # comment {i}\n    start = page * {i}\n    return list(items[start:start + 10])\n\n" for i in range(60))
with tempfile.TemporaryDirectory() as d:
    f = pathlib.Path(d) / "big.py"; f.write_text(src)
    out, err = tool("vtok_read", path=str(f), max_lines=40)
    assert not err and "chunks" in out and "# chunks" in out, out[:300]
    rev = re.search(r"rev=([0-9a-f]{16})", out).group(1)
    cid = re.findall(r"^(c\d+) L\d+-\d+", out, re.M)[0]
    chunk, err = tool("vtok_chunk", path=str(f), id=cid, max_lines=40)
    assert not err
    body = chunk.split("---\n", 1)[1]
    # stale rev rejected
    o, e = tool("vtok_edit", path=str(f), rev="0" * 16, chunks={cid: body}, max_lines=40)
    assert e and "changed since" in o
    # unchanged chunk: no change, file untouched
    o, e = tool("vtok_edit", path=str(f), rev=rev, chunks={cid: body}, max_lines=40)
    assert not e and "no change" in o and f.read_text() == src
    # dry run leaves the file alone, real edit writes it and keeps other chunks (and their comments) intact
    edited = body.replace("page*0", "page*100", 1)
    assert edited != body
    o, e = tool("vtok_edit", path=str(f), rev=rev, chunks={cid: edited}, max_lines=40, dry_run=True)
    assert not e and "dry run" in o and f.read_text() == src
    o, e = tool("vtok_edit", path=str(f), rev=rev, chunks={cid: edited}, max_lines=40)
    assert not e and "written" in o, o
    new = f.read_text()
    assert "page*100" in new.replace(" ", "") and "# comment 59" in new
    # a second edit with the old rev is refused
    o, e = tool("vtok_edit", path=str(f), rev=rev, chunks={cid: edited}, max_lines=40)
    assert e
    # skeleton view, stats, bad paths
    o, e = tool("vtok_read", path=str(f), view="skeleton"); assert not e and "return" not in o.split("---\n", 1)[1]
    o, e = tool("vtok_stats", paths=[str(f), "/nonexistent.py"]); assert "total:" in o and "nonexistent" in o
    o, e = tool("vtok_read", path="/etc/hostname"); assert e
    tiny = pathlib.Path(d) / "t.py"; tiny.write_text("x = 1\n")
    o, e = tool("vtok_read", path=str(tiny)); assert not e and "not be smaller" in o
assert "error" in rpc("nope")
p.stdin.close(); p.wait(timeout=5)
print("mcp ok")
