# Model in the loop: headless Claude Code (`claude -p`), Sonnet, 2026-11-06

`python bench/loop/run.py OUT.json --model sonnet --reps 2 --suite requests|pip --cfgs ...`. Same task, fresh copy of the project, correctness checked by
code (answer match, `ast` truth for the callers questions, import + behaviour test for the edits). We record what the API reports: cost, input tokens
(fresh + cache), output tokens, turns. 14 runs per cell at most; one model; small tasks. Configs: `base` = Read, Grep, Glob, Edit, Bash.
`vtok` = base + Vtok MCP server (instructions and tools available, nothing forced). `vtok+hint` = also the block `vtok init` prints.
`vtok-only` = no built-in file or shell tools, Vtok MCP tools only (forces use). Projects: `requests` (19 files, 8 tasks) and `pip` (495 files, 185k lines, 6 tasks).

| suite | config | correct | cost | input tokens | turns |
|---|---|---|---|---|---|
| requests (16 runs per config) | base | 14/16* | $0.733 | 976k | 46 |
| | vtok | 14/16* | $0.823 | 1057k | 46 |
| | vtok+hint | 14/16* | $0.848 | 1049k | 45 |
| requests (rerun, 2 configs) | base | 16/16 | $0.736 | 999k | 47 |
| | vtok-only | 16/16 | $0.914 | 1340k | 70 |
| pip, first run (3 configs) | base | 12/12 | $0.569 | 781k | 36 |
| | vtok | 12/12 | $0.623 | 778k | 34 |
| | vtok+hint | 12/12 | $0.680 | 961k | 41 |
| pip, vtok-only, server crashed on some calls (see below) | base | 12/12 | $0.562 | 758k | 35 |
| | vtok-only | 12/12 | $0.925 | 1574k | 80 |
| pip, vtok-only, after the fix | base | 12/12 | $0.561 | 757k | 34 |
| | vtok-only | 12/12 | $0.709 | 1102k | 59 |

\* the two failures in the first requests run are a broken checker (the constant is written `DEFAULT_REDIRECT_LIMIT: int = 30`), fixed afterwards.

## What it says

1. **Nothing was gained.** Every task was answered correctly with and without Vtok, and Vtok never made a run cheaper in cost or in total tokens. With the
   tools available but not forced (`vtok`, `vtok+hint`) the model **never called a single Vtok tool in 36 runs**: Grep + Read solved each task in 2-3 turns. The
   extra 8-20% cost is the MCP instructions and tool schemas (about 2k tokens) being carried.
2. **When Vtok is the only way in (`vtok-only`) it costs 24% (requests) to 26% (pip) more**, with 50-70% more turns. The token savings per tool result are real
   (see the other benchmarks) but small next to the fixed per-turn context (about 20k tokens in Claude Code); what moves the bill is the number of turns, and Vtok
   added turns: a `ToolSearch` turn to load the deferred MCP tools, then find, then chunk, then edit.
3. Turning deferred tool loading off (`ENABLE_TOOL_SEARCH=false`) saves the `ToolSearch` turns but loads all tool schemas into every turn: on one task the cost doubled. So the schema weight matters too.
4. The loop found bugs the other benchmarks did not: `vtok find` **panicked on a non-ASCII name** (a string sliced at a byte offset), which killed the MCP server ("Connection closed") on the `pip` tree; and `vtok map` ignored its budget on 487 files (12k characters). Both fixed (`3792f8d`), tool panics are now contained, and regression tests added.

## What it does not say
Small, easy tasks in a tree where `grep` finds the answer at once favour plain tools; tasks that need whole-file reading, long command output
(`vtok run` was not part of these runs), or many files may differ. One model, one harness (headless Claude Code with deferred MCP tools), two projects, correctness
checked by string match. No claim is made about other agents or about large outputs.

## Where this points
Per-call token savings do not pay while turns are the cost. To win in this setting Vtok has to reduce turns, not just tokens: answer "where is X and show me" in one
call (find + code), edit without a separate read, and keep the tool schemas small enough that deferred loading is not needed. Large outputs (tests, builds, big
files) are the case where the per-token savings are largest and the number of turns does not change; that is the next thing to measure.

## caveman (output compression) vs plain, 4 "explain" tasks x 3 runs, Sonnet

`--suite explain --cfgs base,caveman`: the `caveman` config appends the body of caveman's `skills/caveman/SKILL.md` as an extra system prompt (no global install, nothing else
changed). Questions ask for explanations, no "be brief" suffix; correctness = required keywords in the answer.

| config | correct | cost | input tokens | output tokens | turns |
|---|---|---|---|---|---|
| base | 12/12 | $0.681 | 685k | 10,598 | 34 |
| caveman | 12/12 | $0.727 | 707k | 8,475 | 30 |

Output tokens fell **20%**, total cost rose **7%**: the instruction text (about 1.3k tokens) is carried on every turn while answers are a small share of the bill in these
tasks. It is a different lever than Vtok's (what the model writes, not what it reads), the two could be combined, and neither paid for itself here.

## Re-run with the lighter MCP (7 tools, `find show=N`), Sonnet, 2 runs per task

`--cfgs base,vtok-only`, suites pip and requests. Correctness equal (28/28 both). Vtok did **not** pay for itself on these small lookups:

| suite | config | cost | input tokens | turns |
|---|---|---|---|---|
| pip | base | $0.544 | 667k | 30 |
| pip | vtok-only | $0.687 (+26%) | 1005k | 54 |
| requests | base | $0.723 | 931k | 42 |
| requests | vtok-only | $0.909 (+26%) | 1319k | 69 |

Only "callers" tasks tie and one `json method` task wins (1 turn); edit tasks cost 2x the turns (read, then edit, plus the tool-discovery turn). Conclusion: on small
tasks the built-in tools are cheaper in a turn-dominated agent; Vtok's per-call savings only matter on big files / long outputs, which this suite does not exercise.
