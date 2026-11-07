"""Model in the loop: the same tasks, with and without Vtok's MCP tools, run by headless Claude Code (`claude -p`).

Each run gets a fresh copy of a small project (the `requests` package) and one task. We record what the API reports (cost,
input / cache / output tokens, turns) and whether the answer or edit is correct (checked by code, not by a model).
Config `base`: Read, Grep, Glob, Edit, Bash. Config `vtok`: the same plus Vtok's MCP server (`vtok mcp`). Config `vtok+hint`: also the block `vtok init` prints (what a user would put in AGENTS.md).
The task suites (what is asked, and how each answer is checked) are in `suites.py`.
Both use --strict-mcp-config so no other MCP server leaks in. Usage: python bench/loop/run.py OUT.json [--model M] [--reps N] [--only A,B]
Environment: VTOK_BIN (the vtok executable), LOOP_SITE (the site-packages that holds requests, pip and pyparsing), PYTHONUTF8=1 on Windows."""
import argparse, json, os, pathlib, shutil, subprocess, tempfile, concurrent.futures as cf
from suites import tasks

REPO = pathlib.Path(__file__).resolve().parents[2]
SUITE = "requests"
SITE = pathlib.Path(os.environ.get("LOOP_SITE") or REPO / ".venv/lib/python3.12/site-packages")  # where requests and pip are installed
SRC = SITE / "requests"
VTOK = os.environ.get("VTOK_BIN", "/usr/local/bin/vtok")
BASE_TOOLS = "Read,Grep,Glob,Edit,Bash"
VTOK_TOOLS = BASE_TOOLS + "," + ",".join(f"mcp__vtok__vtok_{t}" for t in ("read", "chunk", "edit", "outline", "refs", "find", "map", "output", "stats", "gain"))


def build_big():
    """Prepared once: pyparsing (6k-line core.py) plus fx/ with a failing pytest file (60 passes, 3 failures) and a unittest file."""
    d = pathlib.Path(tempfile.gettempdir()) / "vtok-bigsrc"
    if not (d / "fx" / "test_demo.py").exists():
        shutil.rmtree(d, ignore_errors=True)
        pp = SITE / "pip/_vendor/pyparsing"  # newer pip no longer vendors it: take the installed package
        shutil.copytree(pp if pp.exists() else SITE / "pyparsing", d / "pyparsing", ignore=shutil.ignore_patterns("__pycache__"))
        (d / "fx").mkdir()
        (d / "fx" / "test_demo.py").write_text(
            "import pytest\n\n@pytest.mark.parametrize('n', range(60))\ndef test_ok(n):\n    assert n >= 0\n\n"
            "def test_total_is_wrong():\n    items = [1, 2, 3]\n    assert sum(items) == 7\n\n"
            "def test_missing_key():\n    d = {'a': 1}\n    return d['zzz']\n\n"
            "def test_division():\n    assert 10 / 0\n")
        (d / "fx" / "test_u.py").write_text(
            "import unittest\n\nclass T(unittest.TestCase):\n" + "".join(f"    def test_ok{i}(self):\n        self.assertEqual({i}, {i})\n" for i in range(40)) +
            "    def test_bad_sum(self):\n        self.assertEqual(1 + 1, 3)\n    def test_bad_key(self):\n        {}['k']\n")
    if not (d / "fx" / "test_many.py").exists():
        body = "import pytest\n\ndef helper(x):\n    return inner(x)\n\ndef inner(x):\n    return {'k': 1}[x]\n\n@pytest.mark.parametrize('n', range(900))\ndef test_case(n):\n    assert n * 2 == n + n\n\n"
        body += "".join(f"def test_broken_{i}():\n    helper('missing{i}')\n\n" for i in range(4))
        body += "def test_math():\n    assert round(2.675, 2) == 2.68\n"
        (d / "fx" / "test_many.py").write_text(body)
        (d / "fx" / "test_umany.py").write_text("import unittest\n\nclass T(unittest.TestCase):\n" + "".join(f"    def test_ok{i}(self):\n        self.assertEqual({i}, {i})\n" for i in range(500)) + "    def test_bad_one(self):\n        self.assertEqual('abc'.upper(), 'abd')\n    def test_bad_two(self):\n        self.assertIn(5, [1, 2, 3])\n")
    if not (d / "Makefile").exists():
        (d / "gen_log.py").write_text("import sys\nfor i in range(3000):\n    print(f'[build] compiling module_{i}.c -> module_{i}.o (flags: -O2 -Wall -fPIC) ok in {5 + i % 7}ms')\n    if i == 2222: print('error: module_2222.c:41: undefined reference to `crc32_update` (link failed)')\nprint('build finished: 3000 modules, 1 error')\nsys.exit(1)\n")
        (d / "Makefile").write_text("check:\n\tpython gen_log.py\n")
        pr = d / "proj"; (pr / "tests").mkdir(parents=True)
        (pr / "calc.py").write_text("def clamp(x, lo, hi):\n    return max(lo, min(x, hi - 1))\n\ndef pct(part, whole):\n    return int(part / whole * 100)\n\ndef mean(xs):\n    return sum(xs) / len(xs)\n")
        (pr / "tests" / "test_calc.py").write_text("import pytest\nfrom calc import clamp, pct, mean\n\n@pytest.mark.parametrize('x', range(0, 130))\ndef test_clamp(x):\n    assert clamp(x, 0, 100) == min(x, 100)\n\n@pytest.mark.parametrize('n', range(1, 61))\ndef test_pct(n):\n    assert pct(n, 7) == (n * 100 + 3) // 7\n\ndef test_mean():\n    assert mean([1, 2, 3]) == 2\n")
        (pr / "conftest.py").write_text("import sys, pathlib\nsys.path.insert(0, str(pathlib.Path(__file__).parent))\n")
    return d


CAVEMAN_SKILL = pathlib.Path("/root/workspace/open-source/caveman/skills/caveman/SKILL.md")
CAVEMAN = CAVEMAN_SKILL.read_text().split("---", 2)[-1].strip() if CAVEMAN_SKILL.exists() else ""
CFGS = ("base", "vtok", "vtok+hint")
VTOK_ONLY = ",".join(f"mcp__vtok__vtok_{t}" for t in ("read", "chunk", "edit", "outline", "refs", "find", "map", "output", "stats"))


def one(task, cfg, rep, model, tmp):
    name, prompt, check = task
    wd_root = pathlib.Path(tmp) / f"{cfg}-{rep}-{abs(hash(name)) % 10**6}"
    wd_root.mkdir(parents=True)
    shutil.copytree(SRC, wd_root / SRC.name, ignore=shutil.ignore_patterns("__pycache__"))
    wd = wd_root / SRC.name
    if SUITE == "big": wd = wd_root / "big"; shutil.copytree(SRC, wd, ignore=shutil.ignore_patterns("__pycache__"), dirs_exist_ok=True)
    mcp = wd_root / "mcp.json"
    vt = cfg.startswith("vtok")
    cave = cfg == "caveman"
    only = cfg == "vtok-only"
    mcp.write_text(json.dumps({"mcpServers": {"vtok": {"command": VTOK, "args": ["mcp"], "env": {"VTOK_NO_USAGE": "1"}}} if vt else {}}))  # the server must not log these runs as the user's usage
    if cfg in ("vtok+hint", "vtok-only"): prompt = subprocess.run([VTOK, "init"], capture_output=True, text=True).stdout + "\n" + prompt
    cmd = [os.environ.get("CLAUDE_BIN") or os.environ.get("CLAUDE_CODE_EXECPATH") or "claude", "-p", prompt + ("" if SUITE == "explain" else " Be brief."), "--output-format", "stream-json", "--verbose", "--model", model, "--max-turns", "30", "--no-session-persistence", "--strict-mcp-config",
           "--mcp-config", str(mcp), "--allowedTools", VTOK_ONLY + ",ToolSearch" if only else (VTOK_TOOLS if vt else BASE_TOOLS), "--permission-mode", "acceptEdits"] + (["--disallowedTools", "Read,Grep,Glob,Edit,Write,Bash,NotebookEdit"] if only else []) + (["--append-system-prompt", CAVEMAN] if cave else [])
    env = dict(os.environ, VTOK_NO_USAGE="1", PATH=os.pathsep.join([p for p in ("/usr/local/bin", str(REPO / ".venv/bin")) if os.path.isdir(p)] + [os.environ["PATH"]]))
    if cfg == "hook":
        st = wd_root / "settings.json"
        subprocess.run([VTOK, "init", "--claude-hook", str(st)], capture_output=True)
        cmd += ["--settings", str(st)]
    try:
        r = subprocess.run(cmd, cwd=wd, capture_output=True, text=True, timeout=900, env=env)
        lines = [json.loads(l) for l in r.stdout.splitlines() if l.startswith("{")]
        j = [m for m in lines if m.get("type") == "result"][-1]
        used = {}
        out_chars = sum(len(str(c.get("content", ""))) for m in lines if m.get("type") == "user" for c in (m.get("message", {}).get("content") or []) if isinstance(c, dict) and c.get("type") == "tool_result")
        for m in lines:
            if m.get("type") == "assistant":
                for c in m.get("message", {}).get("content", []):
                    if c.get("type") == "tool_use": used[c["name"]] = used.get(c["name"], 0) + 1
    except Exception as e:
        return {"task": name, "cfg": cfg, "rep": rep, "error": str(e)[:200]}
    u = j.get("usage", {})
    ans = j.get("result", "")
    try: ok = bool(check(wd, ans))
    except Exception as e: ok = False
    tools = j.get("num_turns")
    return {"task": name, "cfg": cfg, "rep": rep, "ok": ok, "cost": j.get("total_cost_usd"), "turns": tools, "answer": ans[:160], "tool_out_chars": out_chars, "tools": used,
            "tokens_in": u.get("input_tokens", 0) + u.get("cache_creation_input_tokens", 0) + u.get("cache_read_input_tokens", 0), "tokens_out": u.get("output_tokens", 0),
            "fresh_in": u.get("input_tokens", 0) + u.get("cache_creation_input_tokens", 0)}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("out")
    ap.add_argument("--model", default="sonnet")
    ap.add_argument("--reps", type=int, default=2)
    ap.add_argument("--jobs", type=int, default=4)
    ap.add_argument("--only", default="")
    ap.add_argument("--cfgs", default="")
    ap.add_argument("--suite", default="requests", choices=["requests", "pip", "explain", "big"])
    a = ap.parse_args()
    global SUITE, SRC
    SUITE = a.suite
    global CFGS
    if a.cfgs: CFGS = tuple(a.cfgs.split(","))
    if SUITE == "big": SRC = build_big()
    if SUITE == "pip": SRC = SITE / "pip"
    ts = [t for t in tasks(SUITE, SRC) if not a.only or any(o in t[0] for o in a.only.split(","))]
    rows = []
    with tempfile.TemporaryDirectory(ignore_cleanup_errors=True) as tmp, cf.ThreadPoolExecutor(a.jobs) as ex:
        futs = [ex.submit(one, t, cfg, rep, a.model, tmp) for t in ts for rep in range(a.reps) for cfg in CFGS]
        for f in cf.as_completed(futs):
            r = f.result(); rows.append(r); print(r.get("task"), r.get("cfg"), r.get("rep"), "ok" if r.get("ok") else ("ERR " + r.get("error", "") if r.get("error") else "FAIL"), r.get("cost"), r.get("turns"), r.get("tools"), flush=True)
    pathlib.Path(a.out).write_text(json.dumps(rows, indent=1))
    print("\n%-24s %-5s %5s %8s %9s %9s %6s" % ("task", "cfg", "ok", "cost$", "in tok", "out tok", "turns"))
    for t in ts:
        for cfg in CFGS:
            rs = [r for r in rows if r["task"] == t[0] and r["cfg"] == cfg and "ok" in r]
            if not rs: continue
            m = lambda k: sum(r[k] or 0 for r in rs) / len(rs)
            print("%-24s %-5s %2d/%-2d %8.4f %9.0f %9.0f %6.1f" % (t[0], cfg, sum(r["ok"] for r in rs), len(rs), m("cost"), m("tokens_in"), m("tokens_out"), m("turns")))
    for cfg in CFGS:
        rs = [r for r in rows if r["cfg"] == cfg and "ok" in r]
        print("TOTAL %-5s ok %d/%d, cost $%.3f, input tokens %d, output tokens %d, turns %d" % (cfg, sum(r["ok"] for r in rs), len(rs), sum(r["cost"] or 0 for r in rs), sum(r["tokens_in"] for r in rs), sum(r["tokens_out"] for r in rs), sum(r["turns"] or 0 for r in rs)))


if __name__ == "__main__":
    main()
