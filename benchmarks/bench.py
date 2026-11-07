"""Compare command-output compaction: raw vs RTK vs vtok vs Caveman's engine, on real commands.

usage: python bench.py [NAME-SUBSTRING...]
Environment (all optional except the binaries you want compared):
  FIXTURES   directory made by fixtures.py (default ./fixtures)
  REPO       repository the git/file scenarios run in (default: this repo)
  RTK_BIN, VTOK_BIN, CAVEMAN_BIN   executables; a missing one is skipped
Tokens are (chars + 3) / 4, the metric RTK's own benchmark uses. "keep" counts how many of a scenario's key strings
(failing test names, error codes, ...) survive in each tool's output: a crude retention check, not a quality score.
"""
import os
import shutil
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
FX = os.path.abspath(os.environ.get("FIXTURES", "fixtures"))
REPO = os.path.abspath(os.environ.get("REPO", os.path.join(HERE, "..")))
BIN = {k: os.environ.get(v) for k, v in (("rtk", "RTK_BIN"), ("vtok", "VTOK_BIN"), ("cave", "CAVEMAN_BIN"))}
env = dict(os.environ, NO_COLOR="1")
env["PATH"] = os.pathsep.join([env["PATH"], os.path.join(FX, "ts", "node_modules", ".bin")])


def tok(s):
    return (len(s) + 3) // 4


def run(cmd, cwd):
    exe = shutil.which(cmd[0], path=env["PATH"]) or cmd[0]
    p = subprocess.run([exe] + cmd[1:], cwd=cwd, capture_output=True, text=True, encoding="utf-8", errors="replace", env=env, timeout=300)
    return p.stdout + p.stderr


def scenarios():
    r, rs = REPO, os.path.join(REPO, "rust")
    return [
        ("git diff (pinned range)", r, ["git", "diff", "3f1264c", "e27cbfc"], ["@@"]),
        ("git log -30", r, ["git", "log", "-30", "e27cbfc"], ["commit"]),
        ("git log --stat -5", r, ["git", "log", "--stat", "-5", "e27cbfc"], []),
        ("ls -la rust", r, ["ls", "-la", "rust"], ["crates"]),
        ("grep fn", FX, ["grep", "-n", "fn ", "big.rs"], ["fn compact_cmd"]),
        ("cat big.rs", FX, ["cat", "big.rs"], ["fn compact_cmd"]),
        ("cargo build err", FX + "/rsfail", ["cargo", "build"], ["E0308", "E0425"]),
        ("cargo test fail", FX + "/rstest", ["cargo", "test"], ["t7", "t19", "t33"]),
        ("cargo test ok", rs, ["cargo", "test", "-p", "vtok-cli"], ["passed"]),
        ("pytest fail", FX + "/py", ["pytest"], ["test_5", "test_17", "test_29"]),
        ("pytest mixed", FX + "/py2", ["pytest"], ["test_case_3", "test_case_9", "test_double", "ZeroDivisionError"]),
        ("mypy", FX + "/py", ["mypy", "m.py"], ["m.py:2"]),
        ("ruff", FX + "/py", ["ruff", "check", "."], ["F401"]),
        ("tsc", FX + "/ts", ["tsc", "--noEmit", "-p", "."], ["TS2322"]),
        ("tsc mixed", FX + "/ts2", ["tsc", "--noEmit", "-p", "."], ["TS2339", "TS2304", "TS2554", "TS2307"]),
        ("eslint", FX + "/ts", ["eslint", "b.js"], ["eqeqeq", "no-var"]),
        ("vitest fail", FX + "/ts", ["vitest", "run"], ["case 4", "case 11"]),
        ("go test fail", FX + "/gof", ["go", "test", "./..."], ["TestF3", "TestF9"]),
    ]


def main(only):
    tot = {"raw": 0, "rtk": 0, "vtok": 0, "cave": 0}
    for name, cwd, cmd, keep in scenarios():
        if only and not any(o in name for o in only):
            continue
        out = {"raw": run(cmd, cwd)}
        if BIN["rtk"]:
            out["rtk"] = run([BIN["rtk"]] + cmd, cwd)
        if BIN["vtok"]:
            out["vtok"] = run([BIN["vtok"], "run", "--"] + cmd, cwd)
        if BIN["cave"]:
            out["cave"] = subprocess.run([BIN["cave"], "compress"], input=out["raw"], capture_output=True, text=True, encoding="utf-8", errors="replace").stdout
        cols = []
        for k, o in out.items():
            tot[k] += tok(o)
            kept = sum(1 for s in keep if s in o)
            cols.append(f"{k}={tok(o):6} ({100 - 100 * tok(o) // max(tok(out['raw']), 1):3}%) keep {kept}/{len(keep)}")
        print(f"{name:18}" + "  ".join(cols), flush=True)
    print("TOTAL " + "  ".join(f"{k}={v} ({100 - 100 * v // max(tot['raw'], 1)}%)" for k, v in tot.items() if v))


if __name__ == "__main__":
    main(sys.argv[1:])
