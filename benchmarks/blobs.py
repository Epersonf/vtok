"""Large repetitive payloads that hide one record, through Caveman's engine and `vtok run -- cat`.

usage: python blobs.py OUTDIR
Environment: CAVEMAN_BIN (caveman-engine), VTOK_BIN (vtok). Look-alikes of the shapes in Caveman's own wrap benchmark
(its corpus generator is not public): a CSV with one outlier amount, a log with one OOMKilled, a YAML with one drifted
image tag, a pytest run with one failure, a JSON with one drifted limit. Deterministic. Both tools get the same bytes.
Tokens are (chars + 3) // 4. "needle" = the hidden record is in the output; "retrieve" = size of `vtok output --grep
NEEDLE`, which an agent can only run when it already knows what to look for.
"""
import json
import os
os.environ.setdefault("VTOK_NO_USAGE", "1")  # benchmark runs must not enter the user's usage log
import random
import subprocess
import sys

OUT = os.path.abspath(sys.argv[1])
CAVE, VTOK = os.environ["CAVEMAN_BIN"], os.environ["VTOK_BIN"]
os.makedirs(OUT, exist_ok=True)
R = random.Random(7)


def tok(s):
    return (len(s) + 3) // 4


def csv_case():
    rows = ["txn_id,timestamp,account_id,merchant,category,amount,currency,country,status"]
    for i in range(1800):
        amt, cty = f"{R.uniform(5, 300):.2f}", R.choice(["US", "US", "US", "GB", "DE", "BR"])
        if i == 1137:
            amt, cty = "48213.77", "XX"
        rows.append(f"TXN-{900000 + i},2026-10-03T{i // 75:02d}:{i % 60:02d}:{(i * 7) % 60:02d}Z,ACC-{R.randint(1000, 1400)},"
                    f"{R.choice(['Acme Mart', 'Blue Cafe', 'City Fuel', 'Net Books'])},{R.choice(['retail', 'food', 'fuel'])},{amt},USD,{cty},settled")
    return "csv", "\n".join(rows) + "\n", "48213.77"


def log_case():
    lines = []
    for i in range(900):
        ts = f"2026-10-03T12:{i // 60:02d}:{i % 60:02d}.{R.randint(100, 999)}Z"
        lines.append(f"{ts} INFO  payments-api[pod=payments-7f9c4d-xk2lp] request_id={R.randint(10**8, 10**9)} method=POST path=/v1/charge status=200 dur_ms={R.randint(20, 90)}")
        if i == 611:
            lines.append(f"{ts} ERROR payments-api[pod=payments-7f9c4d-xk2lp] container=payments-api reason=OOMKilled exit_code=137 limit=512Mi")
    return "log", "\n".join(lines) + "\n", "OOMKilled"


def yaml_case():
    out = ["services:"]
    for i in range(45):
        tag = "1.9.0-rc2" if i == 31 else "1.8.4"
        out += [f"  svc-{i:02d}:", f"    image: registry.local/svc-{i:02d}:{tag}", "    replicas: 3", "    resources:",
                "      requests: {cpu: 250m, memory: 256Mi}", "      limits: {cpu: 500m, memory: 512Mi}", "    env:",
                f"      - name: SERVICE_NAME\n        value: svc-{i:02d}", "      - name: LOG_LEVEL\n        value: info",
                "    probes:", "      liveness: {path: /healthz, port: 8080, period: 10}", "      readiness: {path: /ready, port: 8080, period: 5}",
                f"    labels: {{team: platform, tier: backend, owner: svc-{i:02d}-owner}}"]
    return "yaml", "\n".join(out) + "\n", "1.9.0-rc2"


def pytest_case():
    out = ["============================= test session starts =============================", "collected 900 items", ""]
    for i in range(900):
        name = "test_refund_rounding" if i == 640 else f"test_case_{i % 30:02d}"
        out.append(f"tests/test_module_{i // 30:02d}.py::{name} {'FAILED' if i == 640 else 'PASSED'}" + " " * 14 + f"[{(i + 1) * 100 // 900:3d}%]")
    out += ["", "=================================== FAILURES ===================================",
            "_____________________________ test_refund_rounding _____________________________", "",
            "    def test_refund_rounding():", "        refund = compute_refund(10.005)", ">       assert refund == 10.01", "E       assert 10.0 == 10.01", "",
            "tests/test_module_21.py:14: AssertionError", "=========================== short test summary info ============================",
            "FAILED tests/test_module_21.py::test_refund_rounding - assert 10.0 == 10.01", "======================== 1 failed, 899 passed in 31.20s ========================"]
    return "txt", "\n".join(out) + "\n", "test_refund_rounding"


def json_case():
    items = []
    for i in range(130):
        items.append({"name": f"deploy-{i:03d}", "namespace": "prod", "replicas": 3, "image": f"registry.local/app-{i:03d}:1.8.4",
                      "resources": {"requests": {"cpu": "250m", "memory": "256Mi"}, "limits": {"cpu": "4000m" if i == 88 else "500m", "memory": "512Mi"}},
                      "labels": {"team": "platform", "tier": "backend", "owner": f"app-{i:03d}-owner"},
                      "status": {"ready": 3, "available": 3, "updated": 3, "conditions": [{"type": "Available", "status": "True"}, {"type": "Progressing", "status": "True"}]}})
    return "json", json.dumps({"items": items}, indent=2) + "\n", "4000m"


def text_of(cmd, **kw):
    return subprocess.run(cmd, capture_output=True, **kw).stdout.decode("utf-8", "replace")


env_c = dict(os.environ, HOME=OUT, USERPROFILE=OUT)  # the engine keeps a recovery store under the home directory
print(f"{'payload':10}{'raw':>8}  {'caveman':>8} needle  {'vtok':>6} needle  retrieve")
for make in (csv_case, log_case, yaml_case, pytest_case, json_case):
    ext, text, needle = make()
    name = make.__name__.replace("_case", "")
    path = os.path.join(OUT, f"{name}.{ext}")
    open(path, "w", encoding="utf-8", newline="").write(text)
    c = text_of([CAVE, "compress"], input=text.encode(), env=env_c)
    v = text_of([VTOK, "run", "--", "cat", path])
    rt = ""
    if needle not in v:
        g = text_of([VTOK, "output", "--grep", needle])
        rt = f"{tok(g)}" + ("" if needle in g else " (missing)")
    print(f"{name:10}{tok(text):8}  {tok(c):8} {'yes' if needle in c else 'NO':6}  {tok(v):6} {'yes' if needle in v else 'NO':6}  {rt}")
