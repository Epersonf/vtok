"""Tables for style_eval.py, computed the way Caveman's measure.py does it: Claude's reported output tokens, per-prompt
reduction `1 - arm / control`, and input added = (input + cache-creation + cache-read) of the arm minus the control.

Extra columns: thinking tokens (part of billed output), visible answer length in characters vs control, net tokens per
task (mean output delta + median input delta vs control; the median, because a call that used tools re-reads its
context per turn) and the mean total_cost_usd delta (Claude Code's own price; it depends on prompt-cache warm-up order
between parallel calls, so read it as noise, not as a saving).

Input is a dict {(arm, case_id, rep): cell}, a cell being the compact row style_snapshot.cell() makes: out, think, inp,
cost, chars, turns, failed (list of failed checks, or None when the case has no checks).
"""
import statistics as st

ARM_ORDER = ["baseline", "control", "caveman", "vtok", "vtok-only", "vtok-ultra"]


def _pct(x):
    return f"{'-' if x < 0 else '+'}{abs(x) * 100:.0f}%"


def _arm_line(arm, cells, cells_all):
    outs = [c["out"] for _, _, c in cells]
    paired = [(c, cells_all[("control", cid, rep)]) for cid, rep, c in cells if ("control", cid, rep) in cells_all]
    fid = [not c["failed"] for _, _, c in cells if c["failed"] is not None]
    fid_s = f"{sum(fid)}/{len(fid)}" if fid else "-"
    vs = "- | - | - | - | - | -"
    if arm != "control" and paired:
        red = [1 - c["out"] / k["out"] if k["out"] else 0.0 for c, k in paired]
        din = [c["inp"] - k["inp"] for c, k in paired]
        dout = [c["out"] - k["out"] for c, k in paired]
        dcost = [c["cost"] - k["cost"] for c, k in paired]
        tot = sum(c["out"] for c, _ in paired) / sum(k["out"] for _, k in paired) - 1
        vis = sum(c["chars"] for c, _ in paired) / sum(k["chars"] for _, k in paired) - 1
        vs = (f"{_pct(-st.median(red))} | {_pct(tot)} | {_pct(vis)} | {st.median(din):+.0f} | "
              f"{st.mean(dout) + st.median(din):+.0f} | {st.mean(dcost) * 100:+.3f}c")
    think = st.mean(c["think"] for _, _, c in cells)
    return f"| {arm} | {len(cells)} | {st.median(outs):.0f} | {st.mean(outs):.0f} | {think:.0f} | {vs} | {fid_s} |"


def rows(cells_all, ids):
    """Table lines for the case ids in `ids`."""
    arms = sorted({a for a, cid, _ in cells_all if cid in ids}, key=lambda a: ARM_ORDER.index(a) if a in ARM_ORDER else 99)
    out = ["| arm | n | out median | out mean | thinking mean | vs control, median | vs control, total | visible chars vs control | input added, median | net tok/task | cost/task vs control | fidelity |",
           "|---|---|---|---|---|---|---|---|---|---|---|---|"]
    for arm in arms:
        cells = sorted((cid, rep, c) for (a, cid, rep), c in cells_all.items() if a == arm and cid in ids)
        if cells:
            out.append(_arm_line(arm, cells, cells_all))
    return out


def report(cells_all, langs, title):
    """Markdown: all cases, then English and Portuguese separately, then fidelity failures. langs: {case_id: lang}."""
    out = [f"### {title}", ""]
    for label in ("all", "en", "pt"):
        ids = {cid for cid, lang in langs.items() if label in ("all", lang)}
        if ids:
            out += [f"**{label}** ({len(ids)} prompts)", ""] + rows(cells_all, ids) + [""]
    fails = [f"- {a} · {cid} · rep {rep}: {'; '.join(c['failed'])}"
             for (a, cid, rep), c in sorted(cells_all.items()) if c["failed"]]
    out += ["**fidelity failures**", ""] + (fails or ["- none"]) + [""]
    turns = [f"{a}/{cid}" for (a, cid, _), c in sorted(cells_all.items()) if c["turns"] > 1]
    if turns:
        out += [f"_calls that used tools (num_turns > 1): {', '.join(turns)}_", ""]
    return "\n".join(out)
