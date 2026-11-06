"""Docstring/AST oracle for Rust chunking: force-applying every chunk must reproduce the file's AST
(docstrings included) according to CPython. usage: oracle_chunks.py DIR [limit]"""
import ast, pathlib, subprocess, sys, tempfile, re
rbin = str(pathlib.Path(__file__).parent / "target/release/vtok")
files = [f for f in sorted(pathlib.Path(sys.argv[1]).rglob("*.py")) if len(f.read_text(errors="ignore").splitlines()) > 150]
files = files[: int(sys.argv[2])] if len(sys.argv) > 2 else files
ok = bad = 0
with tempfile.TemporaryDirectory() as tmp:
    for f in files:
        try: want = ast.dump(ast.parse(f.read_text()))
        except Exception: continue
        sp = subprocess.run([rbin, "split", str(f)], capture_output=True, text=True)
        ids = re.findall(r"^(c\d+) L\d+-\d+", sp.stdout, re.M)
        cmd = [rbin, "apply", str(f), "--force", "-o", f"{tmp}/out.py"]
        for i in ids:
            c = subprocess.run([rbin, "chunk", str(f), i], capture_output=True, text=True)
            pth = f"{tmp}/{i}.vtok"; open(pth, "w").write(c.stdout); cmd += ["-e", f"{i}={pth}"]
        r = subprocess.run(cmd, capture_output=True, text=True)
        try: got = ast.dump(ast.parse(open(f"{tmp}/out.py").read()))
        except Exception as e: bad += 1; print("FAIL", f, r.stderr[:120], str(e)[:80]); continue
        if got == want: ok += 1
        else: bad += 1; print("AST DIFF", f)
print(f"ok={ok} bad={bad}")
