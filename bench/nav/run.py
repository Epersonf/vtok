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
The sample is measured in `measure.py`; `mcp_client.py` talks to `vtok mcp`.
"""
import argparse, json, pathlib, random, sys
from mcp_client import Mcp
from measure import measure, summarize

EXT = {"python": (".py",), "rust": (".rs",), "typescript": (".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs")}
BUCKETS = [("150-399", 150, 399), ("400-999", 400, 999), ("1000+", 1000, 10**9)]


def files_of(dirs, lang):
    out = []
    for d in dirs:
        for p in sorted(pathlib.Path(d).rglob("*")):
            if p.suffix in EXT[lang] and p.is_file() and not p.name.endswith(".d.ts") and ".min." not in p.name:
                out.append(p)
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
