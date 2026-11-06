# Self-validation: can a fresh Claude write Vtok? (no API key needed)

Method: for each task, a **fresh subagent** that has seen nothing of this project receives only
`bench/prepare.py`'s prompt (grammar + one example pair + alias table + the file in Vtok + task) and answers with
the whole new file in Vtok. We decode it locally (`bench/check.py`) and run hidden tests. On a parse error the
engine's message goes back to the same agent (retry, K=2), as in spec section 5.
Model: the session's own model (Sonnet 5.5). Arms: Vtok only (raw-code baseline NOT run).

## Round 1 - original grammar, 8 tasks (6 Python, 2 TypeScript)
| | first try | after retry 1 | after retry 2 |
|---|---|---|---|
| decoded OK | 5/8 | 6/8 | 8/8 |
Every reply that decoded passed the hidden tests (8/8): no logic regressions, only notation errors.
Causes of the 3 parse errors (all grammar/prompt faults, not model faults):
1. `E out[k]=v` - inline body with a non-`r/p/B/K/R` statement (decoder too strict, prompt ambiguous) x2
2. `L k item of arr` - `for` header written without parens and with `k` inside (no `for` example in the prompt)
3. a variable named `p` read as the keyword `pass` (retry 1 of flatten) - reserved 1-letter keywords collide
   with the short names a model naturally writes. One agent dodged `for` entirely using `while`.

## Round 2 - revised decoder + prompts, FRESH agents, no retries, 7 tasks (3 re-runs + 4 held-out)
| | first try decoded | tests pass among decoded |
|---|---|---|
| re-runs of round-1 failures (tuned on) | 3/3 | 3/3 |
| held-out new tasks | 3/4 | 3/3 |
The held-out failure: `E ?cond` (else + inline if) instead of `e cond` - same AST, now accepted.

## Reading
- First-try parse failure so far: 4/15 = 27% (spec target < 5%). Small n, and the grammar was being fixed
  between rounds, so this is not a stable estimate - but it shows each round still finds a new idiom.
  Retries rescued all of round 1 within K=2.
- Zero wrong-semantics cases among decoded outputs (14/14 hidden tests). Tasks are easy (20-40 line files).
- On these tiny files the Vtok prompt is ~3x LARGER than the raw prompt (fixed grammar+example ~400 tokens).
  Savings only exist with larger contexts.
- Not measured: raw baseline, other models, large files, chunk workflow, real Claude token counts.
- Cost note: each fresh agent used ~31k tokens (mostly harness overhead), not ~1k.

## Round 3 - cheap check: ONE haiku agent, 4 new held-out tasks, grammar unchanged since round 2
py_slug, py_account, ts_group, ts_queue: **4/4 decoded on the first try and passed the hidden tests**
(41.8k tokens total for the 4, vs ~31k per task for one-agent-per-task).
Caveat: tasks ran sequentially in one agent, so it could learn from earlier ones; smaller model than rounds 1-2.
Observed quirk: it reused the alias `a0` (from the table) as a fresh parameter name - harmless because decode
renames consistently, but a sign aliases can leak into new code.

## Running tally with the revised grammar (fresh agents, no retry)
held-out tasks 7/8 decoded first try (the miss, `E ?cond`, is now accepted); re-runs of earlier failures 3/3.
All decoded outputs of rounds 2-3 (10 of 11 tasks) passed the hidden tests; across all rounds 18/18 decoded outputs were correct in
these rounds. Still n=11-15, one task family: treat as encouraging, not as proof of the < 5% target.

## Raw-code baseline (same haiku model, same 4 tasks as round 3)
Raw: 4/4 pass. Vtok: 4/4 pass. No measurable quality difference - but a ceiling effect (easy tasks, n=4), so this
cannot detect a small degradation. Output tokens are smaller in Vtok (see numbers in the commit message / README);
prompt tokens are larger on these tiny files because of the fixed grammar+example overhead.

## Round 4 (native-tight grammar, haiku, 4 new tasks)

Prompt: the ~100-token native grammar. Tasks target the new options: `py_httpresp` (hidden string
literals, `--hide-literals 5`), `py_guard` (refactor to guard clauses + bug fix), `py_money` (static method
and `__add__` in a class), `ts_listsig` (signature change; `types.ts` supplied as a read-only `lean` context file).

| task | result |
|---|---|
| py_httpresp | pass (placeholders kept, literals restored locally) |
| py_guard | pass |
| py_money | pass |
| ts_listsig | pass |

4/4 on first try, no parse errors, no retries. One agent per task, no raw-code control in this round; small
sample, treat as encouraging. Saved replies: `bench/results/r4_*.txt`.
