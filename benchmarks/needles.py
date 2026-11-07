"""Other kinds of hidden records than the ones the detectors were written around: which survive in each tool's output?

usage: python needles.py OUTDIR      (env CAVEMAN_BIN, VTOK_BIN)
"""
import datetime
import json
import os
import random
import subprocess
import sys

OUT = os.path.abspath(sys.argv[1])
CAVE, VTOK = os.environ["CAVEMAN_BIN"], os.environ["VTOK_BIN"]
os.makedirs(OUT, exist_ok=True)
env_c = dict(os.environ, HOME=OUT, USERPROFILE=OUT)


def tok(s):
    return (len(s) + 3) // 4


def base_rows(n=1800, seed=11, mutate=None):
    r = random.Random(seed)
    rows = []
    for i in range(n):
        row = {"id": f"TXN-{900000 + i}", "ts": (datetime.datetime(2026, 10, 3) + datetime.timedelta(seconds=37 * i)).strftime("%Y-%m-%dT%H:%M:%SZ"), "acct": f"ACC-{r.randint(1000, 1400)}",
               "merchant": r.choice(["Acme Mart", "Blue Cafe", "City Fuel", "Net Books"]), "amount": f"{r.uniform(5, 300):.2f}",
               "country": r.choice(["US", "US", "US", "GB", "DE", "BR"]), "status": "settled", "note": f"order {r.randint(10**6, 10**7)} shipped via {r.choice(['air', 'sea', 'road'])}"}
        rows.append(row)
    if mutate:
        mutate(rows)
    return rows


def to_csv(rows):
    keys = list(rows[0].keys())
    return "\n".join([",".join(keys)] + [",".join(str(r[k]) for k in keys) for r in rows]) + "\n"


def to_json(rows):
    return json.dumps({"items": rows}, indent=2) + "\n"


def to_ndjson(rows):
    return "\n".join(json.dumps(r) for r in rows) + "\n"


def setk(i, **kw):
    def f(rows):
        rows[i].update(kw)
    return f


def dup_id(rows):
    rows[1500]["id"] = rows[200]["id"]


def out_of_order(rows):
    rows[900]["ts"] = "2026-09-01T00:00:00Z"


def extra_key(rows):
    rows[640]["debug_trace"] = "enabled"


def type_drift(rows):
    rows[777]["amount"] = "n/a"


CASES = [
    ("csv", "rare status", to_csv, setk(1300, status="chargeback"), "chargeback"),
    ("csv", "negative amount", to_csv, setk(1300, amount="-500.00"), "-500.00"),
    ("csv", "id of another shape", to_csv, setk(1300, acct="acc_1234"), "acc_1234"),
    ("csv", "blank amount", to_csv, setk(1300, amount=""), "TXN-901300"),
    ("csv", "text in numeric column", to_csv, type_drift, "n/a"),
    ("csv", "duplicate id", to_csv, dup_id, "TXN-900200"),
    ("csv", "timestamp out of order", to_csv, out_of_order, "2026-09-01T00:00:00Z"),
    ("csv", "odd word in free text", to_csv, setk(1300, note="order 5551212 shipped via DRONE"), "DRONE"),
    ("json", "flag flipped", to_json, setk(1300, status="disabled"), "disabled"),
    ("json", "extra key", to_json, extra_key, "debug_trace"),
    ("json", "image from elsewhere", to_json, setk(1300, merchant="evil.example/miner:latest"), "evil.example"),
    ("json", "number as text", to_json, setk(1300, amount="12.50 USD"), "12.50 USD"),
    ("ndjson", "rare status", to_ndjson, setk(1300, status="chargeback"), "chargeback"),
    ("ndjson", "huge amount", to_ndjson, setk(1300, amount="99999.99"), "99999.99"),
]
print(f"{'kind':7}{'needle':26}{'raw':>7}  {'caveman':>8} found   {'vtok':>6} found")
for kind, name, fmt, mut, needle in CASES:
    text = fmt(base_rows(mutate=mut))
    path = os.path.join(OUT, f"{kind}_{name.replace(' ', '_')}.{kind}")
    open(path, "w", encoding="utf-8", newline="").write(text)
    c = subprocess.run([CAVE, "compress"], input=text.encode(), capture_output=True, env=env_c).stdout.decode("utf-8", "replace")
    v = subprocess.run([VTOK, "run", "--", "cat", path], capture_output=True).stdout.decode("utf-8", "replace")
    print(f"{kind:7}{name:26}{tok(text):7}  {tok(c):8} {'yes' if needle in c else 'NO':6}  {tok(v):6} {'yes' if needle in v else 'NO':6}")
