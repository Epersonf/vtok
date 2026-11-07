"""Retention audit through the real binary: for each lint/compiler view, 70 distinct rules or codes (more than any row cap),
the later ones rare; every class must still be named in the output. A class may be grouped or counted, never dropped.
The same check runs inside `cargo test` (src/t_retention.rs); this one drives any built `vtok`, e.g. an older release.
usage: python audit_retention.py VTOK_EXE [--code N]   (--code is for builds that have it: the exit code of the command)"""
import os
import subprocess
import sys

N = 70
exe = os.path.abspath(sys.argv[1])
extra = sys.argv[2:]


def rows(f):
    n = 0
    out = []
    for c in range(N):
        for _ in range(1 + c % 4):
            n += 1
            out.append(f(c, n))
    return out


CASES = [
    ("mypy", "mypy src", lambda c, n: f"src/m.py:{n}: error: Problem {c:02}  [code-{c:02}]", "Found 1 error in 1 file", lambda c: f"code-{c:02}", None),
    ("ruff", "ruff check .", lambda c, n: f"src/a.py:{n}:1: X{c:03} Problem {c:02}", "Found 1 error.", lambda c: f"X{c:03}", None),
    ("oxlint", "oxlint src", lambda c, n: f"  x eslint(rule-{c:02}): Problem\n   ,-[src/a.js:{n}:3]\n 3 | foo();\n   `----", "Found 1 error.", lambda c: f"rule-{c:02}", None),
    ("biome", "biome check src", lambda c, n: f"src/a.ts:{n}:7 lint/correctness/rule{c:02}  ━━━━━━━━━━━━━━━━\n  × Problem\n", "Found 1 error.", lambda c: f"rule{c:02}", None),
    ("rubocop", "rubocop", lambda c, n: f"app/a.rb:{n}:5: C: Style/Cop{c:02}: Problem", "1 file inspected, 1 offense detected", lambda c: f"Cop{c:02}", None),
    ("rubocop json", "rubocop --format json", lambda c, n: '{"severity":"convention","message":"Problem","cop_name":"Style/Cop%02d","correctable":false,"location":{"start_line":%d,"start_column":5}}' % (c, n), "", lambda c: f"Cop{c:02}", "rubocop"),
    ("sqlfluff json", "sqlfluff lint --format json", lambda c, n: '{"line_no":%d,"line_pos":1,"code":"LT%02d","description":"Problem"}' % (n, c), "", lambda c: f"LT{c:02}", "sqlfluff"),
    ("phpstan json", "phpstan analyse --error-format=json", lambda c, n: '{"message":"Problem","line":%d,"identifier":"id.name%02d"}' % (n, c), "", lambda c: f"id.name{c:02}", "phpstan"),
    ("golangci-lint", "golangci-lint run", lambda c, n: f"a.go:{n}:2: Problem (lint{c:02})", "1 issues.", lambda c: f"lint{c:02}", None),
    ("cargo", "cargo build", lambda c, n: f"error[E{c:04}]: Problem {c:02}\n --> src/lib.rs:{n}:5\n  |\n3 | x\n  | ^\n", "error: could not compile `x` due to 1 previous error", lambda c: f"E{c:04}", None),
    ("gcc", "make", lambda c, n: f"a.c:{n}:5: error: Problem {c:02}\n   12 |   foo(bar_baz_qux);\n      |       ^~~~~~~~~~~\n", "make: *** [all] Error 1", lambda c: f"Problem {c:02}", None),
    ("dotnet", "dotnet build", lambda c, n: f"Program.cs({n},5): error CS{1000 + c:04}: Problem [p.csproj]", "Build FAILED.", lambda c: f"CS{1000 + c:04}", None),
    ("mvn", "mvn compile", lambda c, n: f"[ERROR] /src/A.java:[{n},5] Problem {c:02}", "[INFO] BUILD FAILURE", lambda c: f"Problem {c:02}", None),
    ("go build", "go build ./...", lambda c, n: f"./a.go:{n}:2: Problem {c:02}", "", lambda c: f"Problem {c:02}", None),
    ("tsc", "tsc --noEmit", lambda c, n: f"a.ts({n},1): error TS{2000 + c:04}: Problem {c:02}", "", lambda c: f"TS{2000 + c:04}", None),
    ("eslint", "eslint src", lambda c, n: f"  {n}:1  error  Problem {c:02}  rule-{c:02}", "✖ 1 problem (1 error, 0 warnings)", lambda c: f"rule-{c:02}", None),
]


def text(f, tail, json):
    r = rows(f)
    if json == "rubocop":
        return '{"files":[{"path":"app/a.rb","offenses":[%s]}]}' % ",".join(r)
    if json == "sqlfluff":
        return '[{"filepath":"m.sql","violations":[%s]}]' % ",".join(r)
    if json == "phpstan":
        return '{"totals":{"errors":0,"file_errors":%d},"files":{"app/A.php":{"errors":%d,"messages":[%s]}},"errors":[]}' % (len(r), len(r), ",".join(r))
    return ("/app/a.js\n" if f(0, 0).startswith("  0:") or "error  Problem" in f(0, 0) else "") + "\n".join(r) + "\n" + tail + "\n"


bad = 0
for name, cmd, f, tail, marker, json in CASES:
    p = subprocess.run([exe, "filter", "--explain"] + extra + ["--"] + cmd.split(), input=text(f, tail, json), capture_output=True, text=True, encoding="utf-8", errors="replace")
    out = p.stdout
    missing = [c for c in range(N) if marker(c) not in out]
    via = out.splitlines()[0] if out else "?"
    status = "ok" if not missing else f"LOST {len(missing)}/{N} (first {marker(missing[0])})"
    bad += bool(missing)
    print(f"{name:14s} {status:34s} {via}")
print("classes lost in", bad, "of", len(CASES), "views")
