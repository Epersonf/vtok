"""Materialize the self-validation tasks: bench/tasks/<name>/{task.json,src,tests,solution}.

The definitions are the modules of `task_defs` (one per task); this file writes them out."""
import json, pathlib, shutil
from task_defs import T
ROOT = pathlib.Path(__file__).parent / "tasks"

def main():
    if ROOT.exists(): shutil.rmtree(ROOT)
    for name, t in T.items():
        d = ROOT / name
        for sub, files in (("src", t["src"]), ("tests", t["tests"]), ("solution", t["solution"])):
            (d / sub).mkdir(parents=True)
            for fn, body in files.items():
                (d / sub / fn).write_text(body)
        (d / "task.json").write_text(json.dumps({k: t[k] for k in ("lang", "target", "cmd", "instruction", "hide", "context") if k in t}, indent=1))
    print("tasks:", ", ".join(T))

if __name__ == "__main__":
    main()
