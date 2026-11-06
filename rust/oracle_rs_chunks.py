"""Rust chunking oracle: force-applying every chunk must reproduce the file (comments aside).
usage: oracle_rs_chunks.py DIR [max_lines] [limit]"""
import pathlib, subprocess, sys, tempfile, re
rbin = str(pathlib.Path(__file__).parent / "target/release/vtok")
ml = sys.argv[2] if len(sys.argv) > 2 else "150"
files = [f for f in sorted(pathlib.Path(sys.argv[1]).rglob("*.rs")) if len(f.read_text(errors="ignore").splitlines()) > int(ml)]
files = files[: int(sys.argv[3])] if len(sys.argv) > 3 else files
def vt(p):
    r = subprocess.run([rbin, "encode", str(p), "--no-alias"], capture_output=True, text=True)
    return r.stdout if r.returncode == 0 else None
ok = bad = skip = 0
with tempfile.TemporaryDirectory() as tmp:
    for f in files:
        want = vt(f)
        if want is None: skip += 1; continue
        sp = subprocess.run([rbin, "split", str(f), "--max-lines", ml], capture_output=True, text=True)
        ids = re.findall(r"^(c\d+) L\d+-\d+", sp.stdout, re.M)
        cmd = [rbin, "apply", str(f), "--force", "--max-lines", ml, "-o", f"{tmp}/out.rs"]
        for i in ids:
            c = subprocess.run([rbin, "chunk", str(f), i, "--max-lines", ml], capture_output=True, text=True)
            pth = f"{tmp}/{i}.vtok"; open(pth, "w").write(c.stdout); cmd += ["-e", f"{i}={pth}"]
        r = subprocess.run(cmd, capture_output=True, text=True)
        got = vt(f"{tmp}/out.rs")
        if got == want: ok += 1
        else: bad += 1; print("FAIL", f, r.stderr[:150])
print(f"ok={ok} bad={bad} skipped={skip}")
