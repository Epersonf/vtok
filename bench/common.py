import json, pathlib, re, shutil, subprocess, sys, tempfile
import vtok
from vtok import registry, VtokError, render_table

HERE = pathlib.Path(__file__).parent
TASKS = HERE / "tasks"
OUT = pathlib.Path("/tmp/claude-0/-root/b3e99a7a-bed6-489a-b4bf-93d2053b5dd8/scratchpad/bench_out")
FENCE = re.compile(r"```[A-Za-z]*\n(.*?)```", re.S)

def load(name):
    d = TASKS / name
    t = json.loads((d / "task.json").read_text()); t["dir"] = d; t["name"] = name
    t["source"] = (d / "src" / t["target"]).read_text()
    t.setdefault("hide", 0); t.setdefault("context", [])
    t["lits_file"] = OUT / f"{name}.literals.json"
    t["lang_obj"] = registry.get_language(t["lang"])
    t["table"] = vtok.file_table(t["lang_obj"], t["source"])
    return t

def names():
    return sorted(p.name for p in TASKS.iterdir() if p.is_dir())

def extract(reply):
    m = FENCE.findall(reply)
    return (m[-1] if m else reply).rstrip("\n") + "\n"

def run_tests(t, new_source):
    with tempfile.TemporaryDirectory() as tmp:
        tmp = pathlib.Path(tmp)
        for f in (t["dir"] / "src").iterdir():
            shutil.copy(f, tmp / f.name)
        for f in (t["dir"] / "tests").iterdir():
            shutil.copy(f, tmp / f.name)
        (tmp / t["target"]).write_text(new_source)
        if t["cmd"] == "py":
            cmd = [sys.executable, "-m", "pytest", "-q", "-x", "-p", "no:cacheprovider"]
        else:
            tf = [f.name for f in (tmp).iterdir() if f.name.endswith(".test.ts")]
            cmd = ["node", "--experimental-strip-types", "--no-warnings", "--test"] + tf
        try:
            r = subprocess.run(cmd, cwd=tmp, capture_output=True, text=True, timeout=60)
        except subprocess.TimeoutExpired:
            return False, "timeout (infinite loop?)"
        return r.returncode == 0, (r.stdout + r.stderr)[-1500:]
