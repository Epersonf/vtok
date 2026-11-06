"""Differential test: Rust vs Python reference TypeScript encoder. usage: difftest_ts.py DIR [rust_bin]"""
import pathlib, subprocess, sys, tempfile
from vtok import render_table
from vtok_lang_typescript import TypeScript

root = pathlib.Path(sys.argv[1])
rbin = sys.argv[2] if len(sys.argv) > 2 else str(pathlib.Path(__file__).parent / "target/release/vtok")
files = [f for f in sorted(root.rglob("*.ts")) if not f.name.endswith(".d.ts")]
L = TypeScript()
modes = [("full", 0), ("skeleton", 0), ("lean", 0), ("full", 5)]
bad = 0
for view, hide in modes:
    mism = errs = n = 0
    for f in files:
        src = f.read_text()
        try:
            e = L.encode(src, view=view, hide_literals=hide, verify=False)
            want = (e.vtok, render_table(e.table))
        except Exception as ex:
            want = None
        cmd = [rbin, "encode", str(f), "--view", view]
        if hide: cmd += ["--hide-literals", str(hide)]
        r = subprocess.run(cmd, capture_output=True, text=True)
        if want is None:
            errs += (r.returncode == 0)
            continue
        got_tbl = r.stderr.split("# alias table\n", 1)[1].strip() if "# alias table" in r.stderr else ""
        n += 1
        if r.returncode != 0 or r.stdout != want[0] or got_tbl != want[1]:
            mism += 1
            if mism <= 2:
                print("MISMATCH", view, hide, f, r.stderr[:200] if r.returncode else "")
                if r.returncode == 0:
                    a, b = want[0].splitlines(), r.stdout.splitlines()
                    for i, (x, y) in enumerate(zip(a, b)):
                        if x != y: print("  line", i, "\n   py:", x[:160], "\n   rs:", y[:160]); break
                    if got_tbl != want[1]: print("  table differs")
    bad += mism
    print(f"view={view:9} hide={hide}: compared {n}, mismatches {mism}, rust-ok-where-python-errored {errs}")
sys.exit(1 if bad else 0)
