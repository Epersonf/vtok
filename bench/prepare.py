"""Write the prompts a *fresh* model sees: bench/prepare.py [task ...]  (arms: vtok, raw)."""
import sys, pathlib
from common import *
from vtok import ntok

EX = {"python": ("auth.py", "python"), "typescript": ("auth.ts", "typescript")}

def vtok_prompt(t):
    L = t["lang_obj"]; src = t["source"]
    ex_name, fence = EX[t["lang"]]
    ex_src = (HERE.parent / "examples" / ex_name).read_text()
    ex = L.encode(ex_src, alias=False)
    r = L.encode(src, table=t["table"], hide_literals=t["hide"])
    if t["hide"]:
        OUT.mkdir(parents=True, exist_ok=True)
        t["lits_file"].write_text(__import__("json").dumps(r.literals))
    lit_note = "Long string literals in the file are replaced by placeholders such as '$1'; keep them exactly as they are (they are restored automatically). Write new strings in full.\n\n" if t["hide"] else ""
    ctx = ""
    for c in t["context"]:
        cr = L.encode((t["dir"] / "src" / c).read_text(), alias=False, view="lean")
        ctx += f"\n## Context file `{c}` (read-only, lean view: type annotations and import paths removed)\n```vtok\n{cr.vtok}```\n"
    tbl = render_table(t["table"])
    return f"""You edit code written in Vtok, a compact notation. Read the grammar, then do the task.

## Grammar
{L.grammar_prompt}
## Example pair (same project style)
Original:
```{fence}
{ex_src}```
Vtok:
```vtok
{ex.vtok}```

## Alias table (alias=original; use aliases exactly as in the file; write NEW names in full)
{tbl or '(none)'}

## Task
{lit_note}{t['instruction']}

{ctx}
## File `{t['target']}` (Vtok)
```vtok
{r.vtok}```

Reply with ONLY the complete new content of `{t['target']}` in Vtok inside a single ```vtok block. No explanation. Do not use tools or read any files."""

def raw_prompt(t):
    fence = "python" if t["lang"] == "python" else "typescript"
    return f"""You edit code. Do the task below.

## Task
{t['instruction']}

## File `{t['target']}`
```{fence}
{t['source']}```

Reply with ONLY the complete new content of `{t['target']}` in a single ```{fence} block. No explanation. Do not use tools or read any files."""

if __name__ == "__main__":
    OUT.mkdir(parents=True, exist_ok=True)
    sel = sys.argv[1:] or names()
    tot = [0, 0]
    for n in sel:
        t = load(n)
        pv, pr = vtok_prompt(t), raw_prompt(t)
        (OUT / f"{n}.vtok.prompt.txt").write_text(pv); (OUT / f"{n}.raw.prompt.txt").write_text(pr)
        tot[0] += ntok(pv); tot[1] += ntok(pr)
        print(f"{n:16} prompt tokens: vtok={ntok(pv):5} raw={ntok(pr):5}")
    print("TOTAL", tot)
