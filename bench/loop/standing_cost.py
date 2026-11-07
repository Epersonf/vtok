"""The standing cost of registering the vtok MCP server: the prompt size of a one-turn `claude -p` run with no server, with
the default server (8 tools) and with VTOK_MCP_ALL=1 (14 tools). Same tools, same directory, only the server differs; the
size is what the API reports (input + cache write + cache read). `vtok session report` charges this many tokens per turn.

usage: python bench/loop/standing_cost.py ABSOLUTE_VTOK_EXE [--model haiku]
"""
import json
import os
import pathlib
import shutil
import subprocess
import sys
import tempfile


def prompt_size(claude, model, cwd, label, exe, env):
    cmd = [claude, "-p", "Reply with the single word OK.", "--model", model, "--output-format", "json", "--strict-mcp-config",
           "--no-session-persistence", "--max-budget-usd", "0.30"]
    if env is not None:
        cfg = cwd / f"{label}.mcp.json"
        cfg.write_text(json.dumps({"mcpServers": {"vtok": {"command": exe, "args": ["mcp"], "env": dict(env, VTOK_NO_USAGE="1")}}}))
        cmd += ["--mcp-config", str(cfg)]
    u = json.loads(subprocess.run(cmd, capture_output=True, text=True, cwd=cwd, timeout=240).stdout)["usage"]
    total = u["input_tokens"] + u.get("cache_creation_input_tokens", 0) + u.get("cache_read_input_tokens", 0)
    print(f"{label:9} prompt={total:>7}")
    return total


def main():
    exe = sys.argv[1]
    model = sys.argv[sys.argv.index("--model") + 1] if "--model" in sys.argv else "haiku"
    claude = shutil.which("claude")
    cwd = pathlib.Path(tempfile.mkdtemp(prefix="vtok-standing-"))
    sizes = {k: min(prompt_size(claude, model, cwd, f"{k}{i}", exe, env) for i in (1, 2))
             for k, env in (("none", None), ("default", {}), ("all", {"VTOK_MCP_ALL": "1"}))}
    print(f"standing cost per turn: default tool list +{sizes['default'] - sizes['none']} tokens, VTOK_MCP_ALL=1 +{sizes['all'] - sizes['none']} tokens")
    shutil.rmtree(cwd, ignore_errors=True)


if __name__ == "__main__":
    main()
