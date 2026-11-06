"""Oracle test for the Rust Python plugin: decode(encode(x)) must be AST-equal (docstrings stripped) to x
according to CPython's own `ast`. usage: oracle_py.py DIR... [--hide N]"""
import ast, pathlib, subprocess, sys, tempfile, os
sys.path.insert(0, str(pathlib.Path(__file__).parent.parent / "packages/vtok-lang-python/src"))
from vtok_lang_python.encode import strip_docstrings
rbin = str(pathlib.Path(__file__).parent / "target/release/vtok")
args = sys.argv[1:]; hide = 0
if "--hide" in args: i = args.index("--hide"); hide = int(args[i+1]); del args[i:i+2]
files = [f for d in args for f in sorted(pathlib.Path(d).rglob("*.py"))]
ok = bad = skipped = 0; vt_tok = raw_tok = 0
with tempfile.TemporaryDirectory() as tmp:
    for f in files:
        try:
            src = f.read_text(); orig = ast.parse(src); tree = strip_docstrings(ast.parse(src))
            if len(orig.body) == 1 and isinstance(tree.body[0], ast.Pass) and not isinstance(orig.body[0], ast.Pass): tree.body = []  # docstring-only module -> empty
            want = ast.dump(tree)
        except Exception: skipped += 1; continue
        cmd = [rbin, "encode", str(f), "--table", f"{tmp}/t", "--literals", f"{tmp}/l"] + (["--hide-literals", str(hide)] if hide else [])
        for p in (f"{tmp}/t", f"{tmp}/l"):
            if os.path.exists(p): os.remove(p)
        r = subprocess.run(cmd, capture_output=True, text=True)
        if r.returncode:
            bad += 1; print("ENCODE FAIL", f, r.stderr.strip()[:150]); continue
        open(f"{tmp}/v", "w").write(r.stdout)
        dcmd = [rbin, "decode", f"{tmp}/v", "--lang", "python"]
        if os.path.exists(f"{tmp}/t"): dcmd += ["--table", f"{tmp}/t"]
        if os.path.exists(f"{tmp}/l"): dcmd += ["--literals", f"{tmp}/l"]
        d = subprocess.run(dcmd, capture_output=True, text=True)
        try: got = ast.dump(ast.parse(d.stdout))
        except Exception as e: bad += 1; print("DECODE FAIL", f, str(e)[:100]); continue
        if got != want:
            bad += 1; print("AST MISMATCH", f)
        else: ok += 1
print(f"ok={ok} bad={bad} skipped={skipped}")
sys.exit(1 if bad else 0)
