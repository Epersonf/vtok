"""bench/check.py TASK ARM REPLYFILE   |   bench/check.py --solutions  (validate tasks themselves)"""
import json, sys
from common import *

def check(t, arm, reply):
    body = extract(reply)
    if arm == "vtok":
        try:
            lits = json.loads(t["lits_file"].read_text()) if t["hide"] else None
            code = t["lang_obj"].decode(body, {a: o for o, a in t["table"].items()}, lits)
        except VtokError as e:
            return {"status": "parse_error", "detail": str(e), "line": e.line}
        except Exception as e:  # decoder crash == malformed input
            return {"status": "parse_error", "detail": f"{type(e).__name__}: {e}"}
    else:
        code = body
    ok, out = run_tests(t, code)
    return {"status": "pass" if ok else "fail", "detail": "" if ok else out}

if __name__ == "__main__":
    if sys.argv[1] == "--solutions":
        for n in names():
            t = load(n)
            _, base = run_tests(t, t["source"])
            base_ok, _ = run_tests(t, t["source"])
            sol = (t["dir"] / "solution" / t["target"]).read_text()
            sol_ok, sout = run_tests(t, sol)
            # pipeline check: solution -> Vtok (with the task's table) -> decode -> tests
            r = t["lang_obj"].encode(sol, table=t["table"], verify=False, hide_literals=t["hide"])
            if t["hide"]: t["lits_file"].write_text(json.dumps(r.literals))
            res = check(t, "vtok", "```vtok\n" + r.vtok + "```")
            print(f"{n:16} broken_src_fails={not base_ok}  solution_passes={sol_ok}  solution_via_vtok={res['status']}")
            if not sol_ok: print(sout)
            if res["status"] != "pass": print(res["detail"][:300])
    else:
        t = load(sys.argv[1])
        print(json.dumps(check(t, sys.argv[2], open(sys.argv[3]).read())))
