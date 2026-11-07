"""One isolated `claude -p` call per (arm, case, model, repeat), cached on disk, following Caveman's evals/llm_run.py.

Isolation (same flags as Caveman): --setting-sources project (no user settings, so no plugins or their hooks),
--strict-mcp-config (no MCP servers), --disable-slash-commands (no skills), --no-session-persistence, and an empty temp
cwd (no CLAUDE.md found). The system prompt goes through --system-prompt-file and the user prompt through stdin, because
on Windows `claude` is a .CMD shim and cmd.exe mangles multi-line arguments.

Cache: style_cache/<model>/<sha>.json, sha over (arm, system text, prompt, model, repeat); the note text is part of the
system text, so editing the note invalidates only its own arm.
"""
import hashlib
import json
import os
os.environ.setdefault("VTOK_NO_USAGE", "1")  # benchmark runs must not enter the user's usage log
import shutil
import subprocess
import tempfile
import time

HERE = os.path.dirname(os.path.abspath(__file__))
CACHE = os.path.join(HERE, "style_cache")
CONTROL = {"en": "Answer concisely.", "pt": "Responda de forma concisa."}  # Caveman's TERSE_PREFIXES
_CWD = tempfile.mkdtemp(prefix="vtok-style-cwd-")
_SYS = tempfile.mkdtemp(prefix="vtok-style-sys-")


def vtok_note(variant=None):
    exe = os.environ.get("VTOK_BIN") or shutil.which("vtok")
    if not exe:
        raise SystemExit("set VTOK_BIN to the vtok binary whose `init --terse` note you want measured")
    args = [exe, "init", "--terse"] + ([variant] if variant else [])
    return subprocess.run(args, capture_output=True, text=True, encoding="utf-8", check=True).stdout.strip()


def caveman_skill():
    d = os.environ.get("CAVEMAN_DIR")
    if not d:
        raise SystemExit("set CAVEMAN_DIR to a Caveman checkout")
    with open(os.path.join(d, "skills", "caveman", "SKILL.md"), encoding="utf-8") as f:
        return f.read()


ARMS = ("baseline", "control", "caveman", "vtok", "vtok-only", "vtok-ultra")


def arm_texts(arms):
    """The style text each requested arm adds: Caveman's SKILL.md, or the note printed by the vtok binary."""
    t = {}
    if "caveman" in arms:
        t["caveman"] = caveman_skill()
    if "vtok" in arms or "vtok-only" in arms:
        t["vtok"] = vtok_note()
    if "vtok-ultra" in arms:
        t["vtok-ultra"] = vtok_note("ultra")
    return t


def system_text(arm, lang, texts):
    """System prompt for one arm, or None for the baseline (Claude Code's own default system prompt).
    Skill arms are control + text, as in Caveman's method; vtok-only is the note alone (how vtok installs it)."""
    ctl = CONTROL[lang]
    if arm == "baseline":
        return None
    if arm == "control":
        return ctl
    if arm == "vtok-only":
        return texts["vtok"]
    return f"{ctl}\n\n{texts[arm]}"


ISOLATION = "v3"  # part of the cache key: bump when the isolation changes
# Claude Code's built-in commit/PR attribution reminder would add trailer lines to commit-message answers in every arm.
_SETTINGS = os.path.join(_SYS, "settings.json")
with open(_SETTINGS, "w", encoding="utf-8") as _f:
    json.dump({"includeCoAuthoredBy": False, "attribution": {"commit": "", "pr": ""}}, _f)


def child_env():
    """Environment for the child CLI. Caveman's flags are not enough on Claude Code 2.1.3xx: ~/.claude/CLAUDE.md is
    still loaded (user memory is not a setting source) and a child of another Claude session inherits its CLAUDE_*
    variables (user email, session ids, effort, desktop-only tools). Drop those and turn CLAUDE.md and auto-memory off;
    auth still comes from the CLI's own login."""
    env = {k: v for k, v in os.environ.items()
           if not (k.startswith("CLAUDE") or k in ("AI_AGENT", "BAGGAGE"))}
    env.update(CLAUDE_CODE_DISABLE_CLAUDE_MDS="1", CLAUDE_CODE_DISABLE_AUTO_MEMORY="1")
    return env


def key(arm, system, prompt, model, rep):
    raw = json.dumps([arm, system, prompt, model, rep, ISOLATION], ensure_ascii=False)
    return hashlib.sha256(raw.encode("utf-8")).hexdigest()[:24]


def cache_path(model, k):
    return os.path.join(CACHE, model, f"{k}.json")


def cached(model, k):
    p = cache_path(model, k)
    if os.path.exists(p):
        with open(p, encoding="utf-8") as f:
            return json.load(f)
    return None


def call(arm, case, system, model, rep, timeout=300, store=True):
    """Run (or read from cache) one call; returns the stored record with Claude's full JSON result."""
    k = key(arm, system, case["prompt"], model, rep)
    if store and (hit := cached(model, k)) is not None:
        return hit
    cmd = [shutil.which("claude") or "claude", "-p", "--setting-sources", "project", "--strict-mcp-config",
           "--disable-slash-commands", "--no-session-persistence", "--output-format", "json", "--model", model,
           "--settings", _SETTINGS]
    if system is not None:
        sf = os.path.join(_SYS, f"{k}.md")
        with open(sf, "w", encoding="utf-8") as f:
            f.write(system)
        cmd += ["--system-prompt-file", sf]
    for delay in (5, 20, None):
        try:
            out = subprocess.run(cmd, input=case["prompt"], capture_output=True, text=True, encoding="utf-8",
                                 errors="replace", cwd=_CWD, timeout=timeout, env=child_env())
            data = json.loads(out.stdout)
            if data.get("is_error") or not isinstance(data.get("result"), str):
                raise ValueError(f"error result: {out.stdout[-300:]}")
            break
        except (ValueError, subprocess.TimeoutExpired) as e:
            if delay is None:
                raise RuntimeError(f"{arm}/{case['id']}: {e}") from e
            time.sleep(delay)
    rec = {"arm": arm, "case": case["id"], "model": model, "rep": rep, "system": system, "result": data}
    if not store:  # the isolation probe: its answer quotes account context, so it is not written to the repo
        return rec
    os.makedirs(os.path.dirname(cache_path(model, k)), exist_ok=True)
    with open(cache_path(model, k), "w", encoding="utf-8") as f:
        json.dump(rec, f, ensure_ascii=False, indent=1)
    return rec
