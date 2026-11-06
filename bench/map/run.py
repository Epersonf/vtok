"""Orientation: does `vtok map` name the file / symbol an agent should look at, and at what cost?

Questions are author-chosen ("where is X handled?") with a known target file and symbol. For each map budget we check
whether the target FILE and target SYMBOL appear in the output. Baselines: `vtok outline` of every file (always names
everything, expensive) and a bare file listing (names files only). No model: this measures visibility, not whether an
agent would pick the right line. Tokens: tiktoken o200k_base (proxy)."""
import argparse, json, pathlib, subprocess
import tiktoken

ENC = tiktoken.get_encoding("o200k_base")
REPO = pathlib.Path(__file__).resolve().parents[2]
ntok = lambda s: len(ENC.encode(s))

TREES = {
    "vtok": (REPO / "rust", "*.rs *.py", [
        ("outputs stored and garbage-collected", "crates/vtok-cli/src/shell.rs", "gc"),
        ("split a big file into chunks", "crates/vtok-core/src/split.rs", "plan_chunks"),
        ("MCP tool dispatch", "crates/vtok-cli/src/mcp.rs", "call"),
        ("max-lines configuration order", "crates/vtok-cli/src/config.rs", "max_lines"),
        ("alias assignment for names", "crates/vtok-core/src/aliasing.rs", None),
        ("token counting", "crates/vtok-core/src/tokens.rs", "ntok"),
        ("find uses of a symbol across files", "crates/vtok-cli/src/refs.rs", "find"),
        ("Python encoder", "crates/vtok-py/src/lib.rs", "encode"),
        ("usage log and gain report", "crates/vtok-cli/src/usage.rs", "record"),
        ("AGENTS.md block writer", "crates/vtok-cli/src/init.rs", "merge"),
    ]),
    "requests": (REPO / ".venv/lib/python3.12/site-packages/requests", "*.py", [
        ("redirect handling", "sessions.py", "SessionRedirectMixin.resolve_redirects"),
        ("cookie jar", "cookies.py", "RequestsCookieJar"),
        ("basic auth", "auth.py", "HTTPBasicAuth"),
        ("proxy selection", "utils.py", "select_proxy"),
        ("connection pooling adapter", "adapters.py", "HTTPAdapter"),
        ("decode a JSON response", "models.py", "Response.json"),
        ("exception hierarchy", "exceptions.py", "RequestException"),
        ("encode the request body", "models.py", "PreparedRequest.prepare_body"),
    ]),
}


def parse_map(text):
    """{relative path: the map line of that file}"""
    out, cur = {}, ""
    for l in text.splitlines():
        if l.startswith("#"): continue
        if l.endswith("/") and not l.startswith(" "): cur = l; continue
        if l.startswith(" "): out[cur + l.strip().split(" (")[0]] = l
        else: out[l.split(" (")[0]] = l
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("out")
    ap.add_argument("--vtok", default=str(REPO / "rust/target/release/vtok"))
    a = ap.parse_args()
    res = []
    for tname, (root, _, qs) in TREES.items():
        files = [p for p in root.rglob("*") if p.suffix in (".rs", ".py", ".pyi") and "target" not in p.parts and ".venv" not in p.relative_to(root).parts]
        listing = "\n".join(str(p.relative_to(root)) for p in sorted(files))
        outl = ""
        for p in sorted(files):
            o = subprocess.run([a.vtok, "outline", str(p)], capture_output=True, text=True).stdout
            outl += f"{p.relative_to(root)}\n{o}"
        maps = {b: subprocess.run([a.vtok, "map", str(root), "--max-tokens", str(b)], capture_output=True, text=True).stdout for b in (300, 700, 1500)}
        for q, f, sym in qs:
            row = {"tree": tname, "question": q, "file": f, "symbol": sym}
            for b, m in maps.items():
                pm = parse_map(m)
                row[f"file@{b}"] = f in pm
                row[f"symbol@{b}"] = (sym.split(".")[-1] in pm.get(f, "")) if sym else None
            res.append(row)
        res.append({"tree": tname, "question": "(costs)", "listing_tokens": ntok(listing), "all_outlines_tokens": ntok(outl), "map_tokens": {b: ntok(m) for b, m in maps.items()}})
    pathlib.Path(a.out).write_text(json.dumps(res, indent=1))
    for tname in TREES:
        rows = [r for r in res if r["tree"] == tname and r["question"] != "(costs)"]
        c = [r for r in res if r["tree"] == tname and r["question"] == "(costs)"][0]
        print(f"{tname}: {len(rows)} questions; listing {c['listing_tokens']} tok, all outlines {c['all_outlines_tokens']} tok")
        for b in (300, 700, 1500):
            fh = sum(r[f"file@{b}"] for r in rows)
            sy = [r[f"symbol@{b}"] for r in rows if r["symbol"]]
            print(f"  map@{b}: {c['map_tokens'][str(b)] if str(b) in c['map_tokens'] else c['map_tokens'][b]} tok; target file named {fh}/{len(rows)}, target symbol named {sum(sy)}/{len(sy)}")
        for r in rows:
            print("   %-40s %-28s file@700=%s symbol@700=%s" % (r["question"], r["file"].split("/")[-1], r["file@700"], r["symbol@700"]))


if __name__ == "__main__":
    main()
