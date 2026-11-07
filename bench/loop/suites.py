"""Task suites of the model-in-the-loop benchmark: `(name, prompt, check)` triples for `requests`, `pip`, `explain` and `big`.

A check is `check(workdir, answer) -> bool`: the answer or the edit is judged by code, not by a model."""
import ast, os, pathlib, subprocess, sys

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


def tasks_pip(src):
    truth = callers(src, "rmtree_errorhandler")
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


def tasks(suite, src):
    if suite == "big": return tasks_big()
    if suite == "pip": return tasks_pip(src)
    if suite == "explain": return tasks_explain()
    truth_netrc = callers(src, "get_netrc_auth")
    truth_merge = callers(src, "merge_setting")
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
