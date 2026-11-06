"""Navigation benchmark (no LLM, deterministic): what does it cost, in tokens, to *view one symbol* of a file?

For each sampled (file, symbol) it counts the tokens an agent would read under several strategies:

  full        read the whole file (raw)
  grep+exact  `grep -n NAME file` output + the symbol's exact raw lines   (ORACLE: assumes the agent picks the right
              match and knows where the symbol ends - the strongest raw baseline, not what agents really achieve)
  grep+window `grep -n NAME file` output + raw 80-line windows from the definition until the symbol is covered
  vtok index  vtok_read (index + chunk map for files >= 150 lines) then vtok_chunk(symbol)
  vtok skel   vtok_read(view=skeleton) then vtok_chunk(symbol)
  grep+vtok   `grep -n NAME file` output + vtok_chunk(symbol)   (chunk carries the aliases it uses)
  vtok exact  vtok_chunk(symbol) alone (the agent already knows the name)

Vtok numbers are the real `vtok mcp` tool outputs over stdio. Tokens: tiktoken o200k_base (a proxy for Claude's
tokenizer). Raw baselines carry NO line-number prefixes (a real Read tool adds some; that favours the baselines).
Vtok drops comments/docstrings, so it also shows less information - see the 'comments' note in the report.

usage: python bench/nav/run.py OUT.json --corpus LANG=DIR[,DIR] ... [--per-bucket N] [--seed S] [--vtok PATH]
"""
import argparse, json, pathlib, random, re, subprocess, sys
import tiktoken

ENC = tiktoken.get_encoding("o200k_base")
EXT = {"python": (".py",), "rust": (".rs",), "typescript": (".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs")}
BUCKETS = [("150-399", 150, 399), ("400-999", 400, 999), ("1000+", 1000, 10**9)]
SKIP_ID = re.compile(r"^(use|import|imports|IDENT|__)")


def ntok(s):
    return len(ENC.encode(s, disallowed_special=()))


class Mcp:
    def __init__(self, binpath):
        self.p = subprocess.Popen([binpath, "mcp"], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
        self.n = 0
        self.rpc("initialize", {"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "bench", "version": "0"}})

    def rpc(self, method, params):
        self.n += 1
        self.p.stdin.write(json.dumps({"jsonrpc": "2.0", "id": self.n, "method": method, "params": params}) + "\n")
        self.p.stdin.flush()
        return json.loads(self.p.stdout.readline())

    def tool(self, name, **args):
        r = self.rpc("tools/call", {"name": name, "arguments": args})["result"]
        return r["content"][0]["text"], bool(r.get("isError"))


def files_of(dirs, lang):
    out = []
    for d in dirs:
        for p in sorted(pathlib.Path(d).rglob("*")):
            if p.suffix in EXT[lang] and p.is_file() and not p.name.endswith(".d.ts") and ".min." not in p.name:
                out.append(p)
    return out


def symbols(vbin, path):
    r = subprocess.run([vbin, "symbols", str(path)], capture_output=True, text=True)
    if r.returncode:
        return None
    out = []
    for line in r.stdout.splitlines():
        parts = line.rsplit("\t", 2)
        if len(parts) == 3 and parts[1].isdigit() and parts[2].isdigit():
            out.append((parts[0], int(parts[1]), int(parts[2])))
    return out


def grep_cost(lines, name):
    hits = [f"{i + 1}:{l}" for i, l in enumerate(lines) if name in l]
    return ntok("\n".join(hits)), len(hits)


def measure(mcp, vbin, path, lang, rng):
    text = path.read_text(errors="replace")
    lines = text.split("\n")
    syms = symbols(vbin, path)
    if not syms:
        return None
    cand = [(i, a, b) for i, a, b in syms if b - a + 1 >= 5 and not SKIP_ID.match(i) and "#" not in i]
    if not cand:
        return None
    sid, a, b = rng.choice(cand)
    name = re.split(r"[.]", sid)[-1]
    body = "\n".join(lines[a - 1:b])
    g_tok, g_hits = grep_cost(lines, name)
    n = b - a + 1
    window = 80
    win_lines = min(len(lines), -(-n // window) * window)
    read_idx, e1 = mcp.tool("vtok_read", path=str(path))
    read_skel, e2 = mcp.tool("vtok_read", path=str(path), view="skeleton")
    chunk, e3 = mcp.tool("vtok_chunk", path=str(path), id=sid)
    if e1 or e2 or e3:
        return {"error": [read_idx[:100] if e1 else "", chunk[:100] if e3 else ""], "file": str(path), "symbol": sid}
    raw_fallback = read_idx.startswith("#") and "would not be smaller" in read_idx.split("\n", 1)[0]
    t = {
        "full": ntok(text),
        "grep+exact": g_tok + ntok(body),
        "grep+window": g_tok + ntok("\n".join(lines[a - 1:a - 1 + win_lines])),
        "vtok index": ntok(read_idx) + ntok(chunk),
        "vtok skel": ntok(read_skel) + ntok(chunk),
        "grep+vtok": g_tok + ntok(chunk),
        "vtok exact": ntok(chunk),
    }
    return {"file": str(path), "lang": lang, "lines": len(lines), "symbol": sid, "sym_lines": n, "grep_hits": g_hits,
            "raw_fallback": raw_fallback, "tokens": t, "body_tokens": ntok(body)}


def summarize(rows):
    arms = list(rows[0]["tokens"].keys())
    out = {"n": len(rows)}
    for a in arms:
        v = sorted(r["tokens"][a] for r in rows)
        out[a] = {"median": v[len(v) // 2], "mean": round(sum(v) / len(v))}
    for base in ("full", "grep+exact", "grep+window"):
        for a in ("vtok index", "vtok skel", "grep+vtok", "vtok exact"):
            wins = sum(1 for r in rows if r["tokens"][a] < r["tokens"][base])
            ratio = sorted(r["tokens"][a] / max(1, r["tokens"][base]) for r in rows)
            out[f"{a} vs {base}"] = {"wins": f"{wins}/{len(rows)}", "median_ratio": round(ratio[len(ratio) // 2], 2)}
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("out")
    ap.add_argument("--corpus", action="append", required=True, help="LANG=DIR[,DIR]")
    ap.add_argument("--per-bucket", type=int, default=25)
    ap.add_argument("--seed", type=int, default=1)
    ap.add_argument("--vtok", default=str(pathlib.Path(__file__).resolve().parents[2] / "rust/target/release/vtok"))
    a = ap.parse_args()
    rng = random.Random(a.seed)
    mcp = Mcp(a.vtok)
    rows, errors = [], []
    for spec in a.corpus:
        lang, dirs = spec.split("=", 1)
        fs = files_of(dirs.split(","), lang)
        rng.shuffle(fs)
        for label, lo, hi in BUCKETS:
            got = 0
            for f in fs:
                if got >= a.per_bucket:
                    break
                try:
                    nl = f.read_text(errors="replace").count("\n") + 1
                except OSError:
                    continue
                if not lo <= nl <= hi:
                    continue
                r = measure(mcp, a.vtok, f, lang, rng)
                if r is None:
                    continue
                if "error" in r:
                    errors.append(r)
                    continue
                r["bucket"] = label
                rows.append(r)
                got += 1
            print(f"{lang} {label}: {got} tasks", file=sys.stderr)
    report = {"seed": a.seed, "per_bucket": a.per_bucket, "errors": errors, "rows": rows, "summary": {}}
    for lang in sorted({r["lang"] for r in rows}):
        for label, _, _ in BUCKETS:
            sel = [r for r in rows if r["lang"] == lang and r["bucket"] == label]
            if sel:
                report["summary"][f"{lang} {label}"] = summarize(sel)
        report["summary"][f"{lang} all"] = summarize([r for r in rows if r["lang"] == lang])
    report["summary"]["ALL"] = summarize(rows)
    pathlib.Path(a.out).write_text(json.dumps(report, indent=1))
    print(json.dumps(report["summary"]["ALL"], indent=1))


if __name__ == "__main__":
    main()
