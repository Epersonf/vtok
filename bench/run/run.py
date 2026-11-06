"""Terminal-output benchmark (no LLM): does `vtok run` shrink command output while keeping what matters?

Each case runs a real command (some on throw-away fixtures built here: a failing pytest, a failing cargo test, a
compile error, a noisy log). For every case it records raw tokens, tokens shown by `vtok run`, and whether the
KEY FACTS (strings a reader needs: the failing test's name, the error message, the summary line) are still in the
compact view. It also checks that `vtok output ID --grep FACT` recovers each fact from the stored full output.
Tokens: tiktoken o200k_base (proxy). Cases with no key facts (pure listings) only report size.

usage: python bench/run/run.py OUT.json [--vtok PATH] [--max-tokens N]
"""
import argparse, json, os, pathlib, re, subprocess, sys, tempfile
import tiktoken

ENC = tiktoken.get_encoding("o200k_base")
ntok = lambda s: len(ENC.encode(s, disallowed_special=()))
REPO = pathlib.Path(__file__).resolve().parents[2]


def build_fixtures(root):
    r = pathlib.Path(root)
    # failing pytest: 60 parametrized passes + 3 distinct failures
    (r / "pyfail").mkdir()
    (r / "pyfail" / "test_demo.py").write_text(
        "import pytest\n\n@pytest.mark.parametrize('n', range(60))\ndef test_ok(n):\n    assert n >= 0\n\n"
        "def test_total_is_wrong():\n    items = [1, 2, 3]\n    assert sum(items) == 7\n\n"
        "def test_missing_key():\n    d = {'a': 1}\n    return d['zzz']\n\n"
        "def test_division():\n    assert 10 / 0\n")
    # unittest: 40 passing, 2 failing
    (r / "pyunit").mkdir()
    (r / "pyunit" / "test_u.py").write_text(
        "import unittest\n\nclass T(unittest.TestCase):\n" + "".join(f"    def test_ok{i}(self):\n        self.assertEqual({i}, {i})\n" for i in range(40)) +
        "    def test_bad_sum(self):\n        self.assertEqual(1 + 1, 3)\n    def test_bad_key(self):\n        {}['k']\n\nif __name__ == '__main__':\n    unittest.main()\n")
    # failing cargo test among 30 tests
    c = r / "rsfail"
    (c / "src").mkdir(parents=True)
    (c / "Cargo.toml").write_text('[package]\nname = "rsfail"\nversion = "0.1.0"\nedition = "2021"\n')
    tests = "".join(f"    #[test] fn t{i}() {{ assert_eq!({i} + 1, {i + 1}); }}\n" for i in range(30))
    (c / "src" / "lib.rs").write_text("#[cfg(test)]\nmod tests {\n" + tests +
        "    #[test] fn parses_config_value() { assert_eq!(\"42\".parse::<u8>().unwrap() + 1, 44, \"config value off by one\"); }\n}\n")
    # compile error
    e = r / "rserr"
    (e / "src").mkdir(parents=True)
    (e / "Cargo.toml").write_text('[package]\nname = "rserr"\nversion = "0.1.0"\nedition = "2021"\n')
    body = "".join(f"pub fn f{i}(x: u32) -> String {{\n    x + {i}\n}}\n" for i in range(10))
    (e / "src" / "lib.rs").write_text(body + "pub fn g() -> u8 {\n    let s: &str = 5;\n    1\n}\n")
    # noisy log with one buried error
    lines = []
    for i in range(4000):
        lines.append(f"2026-11-06T10:{i // 60 % 60:02d}:{i % 60:02d}Z INFO worker-{i % 7} processed request id={100000 + i} in {5 + i % 9}ms")
        if i == 2711:
            lines += ["2026-11-06T10:45:11Z ERROR worker-3 database connection reset by peer (code 104)",
                      "Traceback (most recent call last):", '  File "app/db.py", line 88, in execute', "    cur.execute(sql)",
                      "psycopg2.OperationalError: server closed the connection unexpectedly"]
    lines.append("2026-11-06T10:59:59Z INFO shutdown complete: 4000 requests, 1 failed")
    (r / "app.log").write_text("\n".join(lines) + "\n")


def cases(fx):
    venv_py = str(REPO / ".venv/bin/python")
    regex_tests = str(REPO / ".venv/lib/python3.12/site-packages/regex/tests")
    return [
        # name, command, cwd, key facts (must survive in the compact view)
        ("cargo test (passing, 3 crates)", "cargo test --release", str(REPO / "rust"), ["16 passed"]),
        ("cargo test (1 failing of 31)", "cargo test", str(fx / "rsfail"), ["parses_config_value", "config value off by one", "FAILED"]),
        ("cargo build (11 compile errors)", "cargo build", str(fx / "rserr"), ["mismatched types", "could not compile", "11 previous errors"]),
        ("pytest -v (failing, 63 tests)", f"{venv_py} -m pytest -v -p no:cacheprovider", str(fx / "pyfail"),
         ["test_total_is_wrong", "test_missing_key", "test_division", "3 failed"]),
        ("pytest -v (regex package, passing)", f"{venv_py} -m pytest -v -p no:cacheprovider {regex_tests} -k 'not test_main'", str(REPO), [" passed"]),
        ("unittest -v (40 pass, 2 fail)", f"{venv_py} -m unittest -v test_u", str(fx / "pyunit"), ["test_bad_sum", "test_bad_key", "FAILED"]),
        ("log: 4000 lines, 1 buried error", f"cat {fx / 'app.log'}", str(REPO),
         ["database connection reset by peer", "OperationalError", "shutdown complete"]),
        ("git log --stat -n 60", "git log --stat -n 60", str(REPO), []),
        ("pip list", f"{venv_py} -m pip list", str(REPO), []),
        ("find site-packages -name '*.py'", f"find {REPO / '.venv/lib/python3.12/site-packages'} -name '*.py'", str(REPO), []),
        ("grep -rn 'def ' (regex pkg)", f"grep -rn 'def ' {REPO / '.venv/lib/python3.12/site-packages/regex'}", str(REPO), []),
    ]


def rtk_cmd(name,cmd,b):
 v=str(b/".venv/bin");sp=str(b/".venv/lib/python3.12/site-packages");m={"cargo test (passing, 3 crates)":"cargo test --release","cargo test (1 failing of 31)":"cargo test","cargo build (11 compile errors)":"cargo build","pytest -v (failing, 63 tests)":"pytest -v -p no:cacheprovider","pytest -v (regex package, passing)":"pytest -v -p no:cacheprovider "+cmd.split("no:cacheprovider ")[-1],"unittest -v (40 pass, 2 fail)":"test "+cmd,"git log --stat -n 60":"git log --stat -n 60","pip list":"pip list","find site-packages -name '*.py'":"find "+sp+" -name '*.py'","grep -rn 'def ' (regex pkg)":"grep -rn 'def ' "+sp+"/regex"}
 if name.startswith("log:"):return "log "+cmd.split("cat ")[-1]
 return m.get(name)

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("out")
    ap.add_argument("--vtok", default=str(REPO / "rust/target/release/vtok"))
    ap.add_argument("--rtk");ap.add_argument("--max-tokens", type=int, default=700)
    a = ap.parse_args()
    rows = []
    with tempfile.TemporaryDirectory(ignore_cleanup_errors=True) as tmp:
        fx = pathlib.Path(tmp) / "fx"
        fx.mkdir()
        build_fixtures(fx)
        env = dict(os.environ, VTOK_STATE=str(pathlib.Path(tmp) / "state"), NO_COLOR="1", CARGO_TERM_COLOR="never")
        for name, cmd, cwd, facts in cases(fx):
            raw = subprocess.run(["sh", "-c", f"{{ {cmd}\n}} 2>&1"], cwd=cwd, capture_output=True, text=True, env=env).stdout
            raw_t = ntok(raw)
            r = subprocess.run([a.vtok, "run", "--max-tokens", str(a.max_tokens), "--", cmd], cwd=cwd, capture_output=True, text=True, env=env)
            shown = r.stdout
            shown_t = ntok(shown)
            m = re.search(r"id=([0-9a-f]+)", shown)
            facts_raw = [f for f in facts if f in raw]
            bad_facts = [f for f in facts if f not in raw]
            facts = facts_raw
            hit = [f for f in facts if f in shown]
            recovered = None
            if m and facts:
                got = 0
                for f in facts:
                    o = subprocess.run([a.vtok, "output", m.group(1), "--grep", f, "--max-tokens", "400"], capture_output=True, text=True, env=env).stdout
                    got += f.lower() in o.lower()
                recovered = f"{got}/{len(facts)}"
            rt = None
            if a.rtk and rtk_cmd(name, cmd, REPO):
                renv = dict(env, PATH=str(REPO / ".venv/bin") + ":" + env["PATH"], RTK_DB_PATH=str(pathlib.Path(tmp) / "rtk.db"), RTK_TELEMETRY_DISABLED="1")
                rr = subprocess.run(["sh", "-c", f"{a.rtk} {rtk_cmd(name, cmd, REPO)}"], cwd=cwd, capture_output=True, text=True, env=renv)
                ro = rr.stdout + rr.stderr
                rt = {"tokens": ntok(ro), "facts": f"{len([f for f in facts if f in ro])}/{len(facts)}", "head": ro[:120]}
            rows.append({"rtk": rt, "case": name, "raw_tokens": raw_t, "shown_tokens": shown_t, "ratio": round(shown_t / max(1, raw_t), 3),
                         "stored": bool(m), "facts_in_view": f"{len(hit)}/{len(facts)}", "facts_missing": [f for f in facts if f not in shown], "invalid_facts_not_in_raw": bad_facts,
                         "recoverable_via_output": recovered})
    pathlib.Path(a.out).write_text(json.dumps(rows, indent=1))
    print("%-40s %9s %9s %6s %9s %11s" % ("case", "raw", "shown", "ratio", "facts", "recoverable"))
    for x in rows:
        print("%-40s %9d %9d %6.2f %9s %11s" % (x["case"], x["raw_tokens"], x["shown_tokens"], x["ratio"], x["facts_in_view"], x["recoverable_via_output"] or "-"))
    if a.rtk:
        print("\nrtk vs vtok (same cases)\n%-40s %7s %7s %7s %10s %9s" % ("case", "raw", "vtok", "rtk", "vtok facts", "rtk facts"))
        for x in rows:
            if x["rtk"]:
                print("%-40s %7d %7d %7d %10s %9s" % (x["case"], x["raw_tokens"], x["shown_tokens"], x["rtk"]["tokens"], x["facts_in_view"], x["rtk"]["facts"]))
    big = [x for x in rows if x["raw_tokens"] > 250]
    med = sorted(x["ratio"] for x in big)[len(big) // 2]
    print("cases with raw > 250 tokens: %d, median shown/raw = %.2f; cases left untouched (not smaller): %d" % (len(big), med, sum(1 for x in rows if x["ratio"] >= 1.0)))
    print("(sum of tokens is dominated by the synthetic log and the listings; read the per-case rows)")
    miss = [(x["case"], x["facts_missing"]) for x in rows if x["facts_missing"]]
    if miss:
        print("facts missing from the compact view:", json.dumps(miss, indent=1))


if __name__ == "__main__":
    main()
