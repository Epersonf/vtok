//! Plugin files for agents that load extensions instead of running a hook command. Each is a thin adapter that asks
//! `vtok rewrite COMMAND` (exit 0 + stdout = rewritten line, exit 1 = leave it) and fails open on any error.

pub const OPENCODE: &str = r#"// vtok: rewrite long-output shell commands to `vtok run -- ...` (fails open)
import { execFile } from "node:child_process"

function rewrite(cmd: string): Promise<string | null> {
  return new Promise((resolve) =>
    execFile("vtok", ["rewrite", cmd], { timeout: 3000, windowsHide: true }, (err, stdout) => resolve(err ? null : stdout.trim() || null)))
}

export default async () => ({
  "tool.execute.before": async (input: any, output: any) => {
    if (input?.tool !== "bash") return
    const cmd = output?.args?.command
    if (typeof cmd !== "string" || process.env.VTOK_OFF) return
    const r = await rewrite(cmd)
    if (r && r !== cmd) output.args.command = r
  },
})
"#;

pub const PI: &str = r#"// vtok: rewrite long-output bash commands to `vtok run -- ...` (fails open). Shared by Pi and Oh My Pi.
export default async function (pi: any) {
  pi.on("tool_call", async (event: any, ctx: any) => {
    try {
      if (event?.toolName !== "bash" || process.env.VTOK_OFF) return
      const cmd = event.input?.command
      if (typeof cmd !== "string" || cmd.trim() === "") return
      const r = await pi.exec("vtok", ["rewrite", cmd], { timeout: 2000, signal: ctx?.signal })
      const out = (r.stdout ?? "").trim()
      if (!r.killed && r.code === 0 && out && out !== cmd) event.input.command = out
    } catch {
      return
    }
  })
}
"#;

pub const HERMES_INIT: &str = r#""""vtok: rewrite Hermes terminal commands to `vtok run -- ...` (fails open)."""
import shutil
import subprocess
import sys


def register(ctx):
    if shutil.which("vtok") is None:
        print("vtok: binary not found in PATH; Hermes hook not registered", file=sys.stderr)
        return
    ctx.register_hook("pre_tool_call", _pre_tool_call)


def _pre_tool_call(tool_name=None, args=None, **_kwargs):
    try:
        if tool_name != "terminal" or not isinstance(args, dict):
            return
        command = args.get("command")
        if not isinstance(command, str) or not command.strip():
            return
        r = subprocess.run(["vtok", "rewrite", command], stdin=subprocess.DEVNULL, capture_output=True, text=True, timeout=5)
        out = r.stdout.strip()
        if r.returncode == 0 and out and out != command:
            args["command"] = out
    except Exception:
        return
"#;

pub const HERMES_YAML: &str = "name: vtok-rewrite\nversion: \"0.1.0\"\ndescription: Rewrite Hermes terminal commands through vtok before execution.\nhooks:\n  - pre_tool_call\nprovides_hooks:\n  - pre_tool_call\n";

pub const ANTIGRAVITY_PLUGIN: &str = "{\n  \"name\": \"vtok\",\n  \"version\": \"0.1.0\",\n  \"description\": \"Compact long shell output with vtok\"\n}\n";

pub const ANTIGRAVITY_HOOKS: &str = "{\n  \"vtok-rewrite\": {\n    \"enabled\": true,\n    \"PreToolUse\": [\n      {\n        \"matcher\": \"run_command\",\n        \"hooks\": [{\"type\": \"command\", \"command\": \"vtok hook --agent antigravity\", \"timeout\": 10}]\n      }\n    ]\n  }\n}\n";

pub const VIBE_ENTRY: &str = "[[hooks]]\nname = \"vtok-rewrite\"\ntype = \"pre_tool\"\nmatch = \"bash\"\ncommand = \"vtok hook --agent vibe\"\ntimeout = 10.0\nstrict = false\ndescription = \"Compact long shell output with vtok.\"\n";
