"""One (file, symbol) sample: the tokens an agent reads under each strategy, and the per-bucket summary of many samples."""
import re, subprocess
import tiktoken

ENC = tiktoken.get_encoding("o200k_base")
SKIP_ID = re.compile(r"^(use|import|imports|IDENT|__)")


def ntok(s):
    return len(ENC.encode(s, disallowed_special=()))


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
