"""Model in the loop: the same tasks, with and without Vtok's MCP tools, run by headless Claude Code (`claude -p`).

Each run gets a fresh copy of a small project (the `requests` package) and one task. We record what the API reports (cost,
input / cache / output tokens, turns) and whether the answer or edit is correct (checked by code, not by a model).
Config `base`: Read, Grep, Glob, Edit, Bash. Config `vtok`: the same plus Vtok's MCP server (`vtok mcp`). Config `vtok+hint`: also the block `vtok init` prints (what a user would put in AGENTS.md).
Both use --strict-mcp-config so no other MCP server leaks in. Usage: python bench/loop/run.py OUT.json [--model M] [--reps N]"""
import argparse, ast, json, os, pathlib, shutil, subprocess, sys, tempfile, concurrent.futures as cf

REPO = pathlib.Path(__file__).resolve().parents[2]
SUITE = "requests"
SRC = REPO / ".venv/lib/python3.12/site-packages/requests"
VTOK = "/usr/local/bin/vtok"
BASE_TOOLS = "Read,Grep,Glob,Edit,Bash"
VTOK_TOOLS = BASE_TOOLS + "," + ",".join(f"mcp__vtok__vtok_{t}" for t in ("read", "chunk", "edit", "outline", "refs", "find", "map", "output", "stats", "gain"))


def callers(root, name):
    out = set()
    for f in pathlib.Path(root).rglob("*.py"):
        try: tree = ast.parse(f.read_text())
        except Exception: continue
        for fn in ast.walk(tree):
            if isinstance(fn, (ast.FunctionDef, ast.AsyncFunctionDef)) and fn.name != name:
                for n in ast.walk(fn):
                    if (isinstance(n, ast.Name) and n.id == name) or (isinstance(n, ast.Attribute) and n.attr == name): out.add(fn.name); break
    return out


def names_in(text):
    import re
    return set(re.findall(r"[A-Za-z_][A-Za-z0-9_]*", text))


def tasks_pip():
    truth = callers(SRC, "rmtree_errorhandler")
    both = lambda *ws: (lambda wd, ans: all(w in ans for w in ws))
    def edit_size(wd, ans):
        r = subprocess.run([sys.executable, "-c", "from pip._internal.utils.misc import format_size as f; assert f(-5)=='n/a' and f(1500)=='1.5 kB' and f(2*1000*1000)=='2.0 MB'"], cwd=wd.parent, capture_output=True, env=dict(os.environ, PYTHONPATH=str(wd.parent)))
        return r.returncode == 0
    return [
        ("pip: bytes to text", "Which function turns a byte count into human readable text such as '1.5 kB'? Reply only `file:function`.", both("misc.py", "format_size")),
        ("pip: download cmd", "Which class implements the `pip download` command? Reply only `file:Class`.", both("download.py", "DownloadCommand")),
        ("pip: parse req line", "Which function in the req constructors module parses a requirement line into its parts (name, url, extras)? Reply only `file:function`.", both("constructors.py", "parse_req_from_line")),
        ("pip: callers errorhandler", "List the names of all functions that call or reference `rmtree_errorhandler` anywhere in this package. Reply with a comma-separated list of function names only.", lambda wd, ans: truth <= names_in(ans)),
        ("pip: where is is_url", "In which file is the function `is_url` defined, and what does it return for an empty string? Reply `file: value`.", both("versioncontrol.py")),
        ("pip: edit format_size", "In _internal/utils/misc.py (relative to the current directory) make `format_size` return the string 'n/a' when its argument is negative; keep everything else the same. Reply `done` when finished.", edit_size),
    ]


def tasks_explain():
    kw = lambda *groups: (lambda wd, ans: all(any(k.lower() in ans.lower() for k in g) for g in groups))
    return [
        ("explain redirects", "Explain what Session.resolve_redirects does and how it stops following redirects forever. Answer in a short paragraph.", kw(("redirect",), ("TooManyRedirects", "max_redirects"))),
        ("explain select_proxy", "Explain how `select_proxy` in utils.py chooses a proxy for a URL: which keys of the proxies dict does it try, in which order?", kw(("scheme",), ("hostname",), ("all",))),
        ("explain netrc", "Explain what `get_netrc_auth` does and when it returns None.", kw(("netrc",), ("None",))),
        ("explain json", "Explain how Response.json decides which encoding to use and what it raises on invalid JSON.", kw(("encoding", "utf"), ("JSONDecodeError", "RequestsJSONDecodeError"))),
    ]


def build_big():
    """Prepared once: pyparsing (6k-line core.py) plus fx/ with a failing pytest file (60 passes, 3 failures) and a unittest file."""
    d = pathlib.Path("/tmp/vtok-bigsrc")
    if not (d / "fx" / "test_demo.py").exists():
        shutil.rmtree(d, ignore_errors=True)
        shutil.copytree(REPO / ".venv/lib/python3.12/site-packages/pip/_vendor/pyparsing", d / "pyparsing", ignore=shutil.ignore_patterns("__pycache__"))
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


def tasks_big():
    both = lambda *ws: (lambda wd, ans: all(w in ans for w in ws))
    core = lambda wd: (wd / "pyparsing" / "core.py").read_text()
    def edit_white(wd, ans):
        return 'DEFAULT_WHITE_CHARS: str = " \\t"' in core(wd) and subprocess.run([sys.executable, "-c", "import pyparsing"], cwd=wd, capture_output=True).returncode == 0
    def edit_default(wd, ans):
        t = core(wd); i = t.index("def scan_string(")
        return "max_matches: int = 1000," in t[i:i + 300] and subprocess.run([sys.executable, "-c", "import pyparsing"], cwd=wd, capture_output=True).returncode == 0
    def fix_calc(wd, ans):
        return subprocess.run([sys.executable, "-m", "pytest", "-q", "-p", "no:cacheprovider", "proj"], cwd=wd, capture_output=True, env=dict(os.environ, PYTHONDONTWRITEBYTECODE="1")).returncode == 0
    return [
        ("hard: build log", "Run `make check` and report the first error line and the final summary line.", both("crc32_update")),
        ("hard: fix tests", "The tests in proj/ fail (run them from the repo root with `python -m pytest proj`). Fix the bug in proj/calc.py, not the tests, until they pass. Reply `done`.", fix_calc),
        ("big: default arg", "In pyparsing/core.py, what is the default value of the `max_matches` parameter of `ParserElement.scan_string`? Reply only the expression.", both("_MAX_INT")),
        ("big: where defined", "In pyparsing/core.py, which line range (approximately) does the method `ParserElement.transform_string` occupy? Reply `start-end` line numbers.", lambda wd, ans: any(abs(int(a) - 1285) <= 2 for a in __import__("re").findall(r"\d+", ans)[:1])),
        ("big: edit constant", "In pyparsing/core.py change the value of DEFAULT_WHITE_CHARS (the class attribute at the top of ParserElement) from \" \\n\\t\\r\" to \" \\t\". Reply `done` when finished.", edit_white),
        ("big: edit default", "In pyparsing/core.py change the default of `max_matches` in `ParserElement.scan_string` from `_MAX_INT` to `1000`. Reply `done` when finished.", edit_default),
        ("big: pytest failures", "Run `python -m pytest -v -p no:cacheprovider fx/test_demo.py` and list the failing test names with the failing assertion or error of each. Reply as `name: reason` lines.", both("test_total_is_wrong", "test_missing_key", "test_division")),
        ("big: pytest 900 tests", "Run `python -m pytest -v -p no:cacheprovider fx/test_many.py` and list each failing test name with its error. Reply as `name: reason` lines.", both("test_broken_0", "test_broken_3", "test_math")),
        ("big: unittest 500 tests", "Run `python -m unittest -v fx.test_umany` and list the failing tests with the error of each. Reply as `name: reason` lines.", both("test_bad_one", "test_bad_two")),
        ("big: unittest failures", "Run `python -m unittest -v fx.test_u` and list the failing tests with the error of each. Reply as `name: reason` lines.", both("test_bad_sum", "test_bad_key")),
    ]


def tasks():
    if SUITE == "big": return tasks_big()
    if SUITE == "pip": return tasks_pip()
    if SUITE == "explain": return tasks_explain()
    truth_netrc = callers(SRC, "get_netrc_auth")
    truth_merge = callers(SRC, "merge_setting")
    def q_set(truth, extras_ok=1):
        return lambda wd, ans: truth <= names_in(ans) and len([n for n in names_in(ans) if n.islower() and "_" in n and n not in truth]) <= extras_ok + 6
    def edit_limit(wd, ans):
        t = (wd / "models.py").read_text()
        return __import__("re").search(r"DEFAULT_REDIRECT_LIMIT\s*(:\s*int)?\s*=\s*25", t) and not __import__("re").search(r"DEFAULT_REDIRECT_LIMIT\s*(:\s*int)?\s*=\s*30", t) and subprocess.run([sys.executable, "-c", "import requests"], cwd=wd.parent, capture_output=True, env=dict(os.environ, PYTHONPATH=str(wd.parent))).returncode == 0
    def edit_param(wd, ans):
        t = ast.parse((wd / "utils.py").read_text())
        fn = [n for n in ast.walk(t) if isinstance(n, ast.FunctionDef) and n.name == "select_proxy"][0]
        ok_args = [a.arg for a in fn.args.args][:1] == ["target_url"] and not any(isinstance(n, ast.Name) and n.id == "url" for n in ast.walk(fn))
        r = subprocess.run([sys.executable, "-c", "import requests.utils as u; assert u.select_proxy('http://a.com/x', {'http':'P'})=='P'"], cwd=wd.parent, capture_output=True, env=dict(os.environ, PYTHONPATH=str(wd.parent)))
        return ok_args and r.returncode == 0
    both = lambda *ws: (lambda wd, ans: all(w in ans for w in ws))
    return [
        ("find proxy fn", "Which function chooses the proxy for a URL from a proxies dict? Reply only `file:function`.", both("utils.py", "select_proxy")),
        ("redirect limit", "Which function raises TooManyRedirects when too many redirects happened? Reply only `file:function`.", both("sessions.py", "resolve_redirects")),
        ("json method", "Which class and method decode a response body as JSON? Reply only `Class.method`.", both("Response", "json")),
        ("basic auth header", "Which function builds the Basic Authorization header string? Reply only `file:function`.", both("auth.py", "_basic_auth_str")),
        ("callers netrc", "List the names of all functions (not methods' classes) that call or reference `get_netrc_auth` anywhere in this package. Reply with a comma-separated list of function names only.", q_set(truth_netrc)),
        ("callers merge_setting", "List the names of all functions that call or reference `merge_setting` anywhere in this package. Reply with a comma-separated list of function names only.", q_set(truth_merge)),
        ("edit constant", "In models.py change the value of DEFAULT_REDIRECT_LIMIT from 30 to 25. Reply `done` when finished.", edit_limit),
        ("edit param", "In utils.py rename the first parameter of the function `select_proxy` from `url` to `target_url`, updating its uses inside that function. Reply `done` when finished.", edit_param),
    ]


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
    mcp.write_text(json.dumps({"mcpServers": {"vtok": {"command": VTOK, "args": ["mcp"]}} if vt else {}}))
    if cfg in ("vtok+hint", "vtok-only"): prompt = subprocess.run([VTOK, "init"], capture_output=True, text=True).stdout + "\n" + prompt
    cmd = [os.environ.get("CLAUDE_BIN") or os.environ.get("CLAUDE_CODE_EXECPATH") or "claude", "-p", prompt + ("" if SUITE == "explain" else " Be brief."), "--output-format", "stream-json", "--verbose", "--model", model, "--max-turns", "30", "--no-session-persistence", "--strict-mcp-config",
           "--mcp-config", str(mcp), "--allowedTools", VTOK_ONLY + ",ToolSearch" if only else (VTOK_TOOLS if vt else BASE_TOOLS), "--permission-mode", "acceptEdits"] + (["--disallowedTools", "Read,Grep,Glob,Edit,Write,Bash,NotebookEdit"] if only else []) + (["--append-system-prompt", CAVEMAN] if cave else [])
    env = dict(os.environ, VTOK_NO_USAGE="1", PATH="/usr/local/bin:" + str(REPO / ".venv/bin") + ":" + os.environ["PATH"])
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
    if SUITE == "explain": SRC = REPO / ".venv/lib/python3.12/site-packages/requests"
    if SUITE == "big": SRC = build_big()
    if SUITE == "pip": SRC = REPO / ".venv/lib/python3.12/site-packages/pip"
    ts = [t for t in tasks() if not a.only or a.only in t[0]]
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
