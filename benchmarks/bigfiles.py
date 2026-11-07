"""Speed and memory of `vtok run -- cat FILE` on very large data files, and whether the one odd record survives.

usage: python bigfiles.py OUTDIR [NAME]     (env VTOK_BIN; `pip install psutil` adds the peak memory column)
Makes CSV, JSON (pretty, minified on one line), NDJSON and YAML files of 15k to 250k records in OUTDIR (reused when they
exist; NAME keeps only the cases whose label contains it), hides one amount of 48213.77 in each, and times the view.
Run it with another binary in VTOK_BIN to compare two builds. Deterministic apart from timings.
"""
import csv
import json
import os
import random
import subprocess
import sys
import time

VT = os.environ["VTOK_BIN"]
OUT = os.path.abspath(sys.argv[1])
ONLY = sys.argv[2] if len(sys.argv) > 2 else ""
NEEDLE = "48213.77"
os.makedirs(OUT, exist_ok=True)
R = random.Random(7)


def rec(i):
    return {"id": f"T{100000 + i}", "ts": f"2026-03-01T{(i * 37 // 3600) % 24:02d}:{(i * 37 // 60) % 60:02d}:{i * 37 % 60:02d}Z",
            "user": f"user{R.randint(1, 9000)}", "country": R.choice(["US", "DE", "GB", "BR", "FR", "JP", "IN", "CA"]),
            "status": R.choice(["ok", "ok", "ok", "ok", "retry", "queued"]), "amount": round(R.lognormvariate(3, 0.6), 2),
            "items": R.randint(1, 9), "latency_ms": R.randint(5, 400), "region": R.choice(["east", "west", "north"]), "note": ""}


def records(n):
    rows = [rec(i) for i in range(n)]
    rows[n // 2]["amount"] = float(NEEDLE)
    return rows


def write_csv(path, n):
    rows = records(n)
    with open(path, "w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=list(rows[0]))
        w.writeheader()
        w.writerows(rows)


def write_json(path, n, indent):
    with open(path, "w") as f:
        json.dump(records(n), f, indent=indent)


def write_ndjson(path, n):
    with open(path, "w") as f:
        f.write("\n".join(json.dumps(r) for r in records(n)) + "\n")


def write_yaml(path, n):
    with open(path, "w") as f:
        for r in records(n):
            f.write("- " + "\n  ".join(f"{k}: {json.dumps(v)}" for k, v in r.items()) + "\n")


CASES = [
    ("csv 150k", "big150.csv", lambda p: write_csv(p, 150_000)),
    ("csv 250k", "big250.csv", lambda p: write_csv(p, 250_000)),
    ("json 18k, pretty", "big18k.json", lambda p: write_json(p, 18_000, 2)),
    ("json 150k, one line", "big150k.json", lambda p: write_json(p, 150_000, None)),
    ("ndjson 190k", "big190k.ndjson", lambda p: write_ndjson(p, 190_000)),
    ("yaml 15k", "big15k.yaml", lambda p: write_yaml(p, 15_000)),
]


def timed(path):
    start = time.perf_counter()
    p = subprocess.Popen([VT, "run", "--", "cat", path], stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    peak = 0.0
    try:
        import psutil
        ps = psutil.Process(p.pid)
        while p.poll() is None:
            try:
                peak = max(peak, ps.memory_info().peak_wset / 1e6)
            except Exception:
                pass
            time.sleep(0.02)
    except ImportError:
        pass
    out, _ = p.communicate()
    return time.perf_counter() - start, peak, out.decode("utf-8", "replace")


print(f"{'case':20}{'size MB':>8}{'seconds':>9}{'peak MB':>9}{'out tok':>9}  needle")
for label, name, make in CASES:
    if ONLY not in label:
        continue
    path = os.path.join(OUT, name)
    if not os.path.exists(path):
        make(path)
    secs, peak, text = timed(path)
    print(f"{label:20}{os.path.getsize(path) / 1e6:8.1f}{secs:9.2f}{peak:9.0f}{(len(text) + 3) // 4:9}  {'yes' if NEEDLE in text else 'NO'}")
