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

## Large files and long outputs (suite `big`), base vs `hook`, Sonnet, 3 runs per task

`bench/loop/run.py --suite big --cfgs base,hook`: pyparsing `core.py` (6.1k lines) lookups and edits, and pytest / unittest runs of 63 to 900 tests with a few failures. `hook` = `vtok init --claude-hook`.

| config | correct | cost | input tokens | turns |
|---|---|---|---|---|
| base | 24/24 | $1.184 | 1398k | 62 |
| hook | 24/24 | $1.228 | 1471k | 63 |

No cost difference (the gap is the `edit default` task, which has nothing to do with the hook). Two reasons, both measured: the agent already adds `| tail -60` to long
commands (the hook skips piped commands), and Claude Code already stores very large tool output on disk and shows a preview. On a 63-test pytest run the compact view is
~110 tokens against ~1.8k raw (`tool_out_chars` in the results), a saving of cents at most against a ~45k-token fixed context per run. Separately, in the same suite the
agent never called Vtok's MCP tools even with the usage block in the prompt: its own Grep and ranged Read were enough on a 6k-line file.

## Scenarios where the agent cannot just `| tail` (suite `big`, tasks `hard:`), Sonnet

`hard: build log`: `make check` prints 3000 compile lines with one linker error in the middle; the question is the first error line. Correct answers out of 6 runs:

| config | correct | input tokens | turns |
|---|---|---|---|
| base | 2/6 | 376k | 17 |
| hook (`vtok init --claude-hook`) | 4/6 | 308k | 14 |
| vtok+hint (agent calls `vtok run`) | 6/6 | 283k | 12 |

Here the compact view wins on correctness (the agent's own `head`/`tail` often misses the buried error, and twice reported "no error") and uses about 25% fewer input tokens; cost is about equal because of the fixed context.

`hard: fix tests` (56 failing parametrized tests, fix `calc.py`): 4/4 for every config, no saving, the agent pipes through `tail` itself and vtok+hint adds turns. This run exposed a gap that was fixed:
plain `pytest` failure reports (no `-v`) with many parametrized failures were not compacted (2.5k tokens of near-identical blocks); `vtok run` now keeps the first block of each
test function plus any with a different assertion shape and says `+29 more failures of test_clamp with the same assertion (101, 102, ... 129)` (about 170 tokens).

`big: edit constant` / `edit default` on a 6.1k-line file, `vtok-only` vs native Edit: first run 0/6 correct. Cause (a product bug, not the benchmark): Python edits rewrote the edited
line in Vtok's compact spacing (`X:str=" \t"` instead of `X: str = " \t"`) and dropped comments, because the Python plugin had no token-level patching (TS and Rust did). Python now
patches at token level, strings as single tokens, and falls back to it for single-statement and class-attribute chunks. Result: 6/6 correct, original formatting and comments kept.
Cost is still higher than native Edit in that harness (about 7 turns against 3: tool discovery, a chunk read, then the edit), so for one-line edits the native tool stays cheaper.

## Windows re-run with the 6f51c4e build (plain-text fallback, `replace`, 370-token instructions), Sonnet, 2026-11-07

`PYTHONUTF8=1 VTOK_BIN=C:/Users/epers/.vtok/bin/vtok.exe LOOP_SITE=<python>/Lib/site-packages python bench/loop/run.py OUT.json --suite requests --cfgs base,vtok,vtok-only --reps 2`,
and the same with `--suite big --only "default arg,edit"`. Windows 11, `requests` 2.34.2 and the installed `pyparsing` (its `core.py` has 7.0k lines, the runs above used one with 6.1k),
so cells do not compare one to one with the tables above. Two runs per cell, one model: read the totals, not single cells. Results: `results-win-requests.json`, `results-win-big.json`.

| suite | config | correct | cost | input tokens | turns | Vtok tool calls |
|---|---|---|---|---|---|---|
| requests (8 tasks x 2) | base | 16/16 | $0.974 | 2.00M | 50 | 0 |
| | vtok | 16/16 | $1.003 (+3.0%) | 1.91M | 46 | 0 |
| | vtok-only | 16/16 | $1.061 (+8.9%) | 1.99M | 61 | 23 |
| big (1 lookup and 2 edits in `core.py`, x 2) | base | 6/6 | $0.359 | 0.69M | 18 | 0 |
| | vtok | 6/6 | $0.401 (+11.7%) | 0.83M | 22 | 0 |
| | vtok-only | 6/6 | $0.643 (+79%) | 1.12M | 35 | 18 |

What the standing cost is, measured separately with one-turn runs (`claude -p "Reply with the single word OK."`, the prompt size the API reports, same tools, only the MCP server differs):
no server 40,384 tokens, the 2026-11-06 build 42,054 (+1,669), this build 41,114 (+729). Those tokens ride on every turn of every session that has the server registered, used or not.

1. **Having the server registered now costs 3% on small tasks, down from 12%** ($0.823 against $0.733 before). The server is still never used when the agent has Grep, Read and Edit: 0 of 22 runs here (0 of 36 before).
2. **Forced use costs 9% more on small tasks, down from 24%.** The plain-text fallback is why: where the compact form does not pay, the tools answer in plain text (the chunk of `DEFAULT_WHITE_CHARS` came back as the one plain line). It still adds turns (14 `ToolSearch` turns in 16 runs).
3. **On the big file forced use is still dearer** (+79%, but one `edit constant` run took 10 turns and $0.256 and its twin 5 turns and $0.076: that spread is larger than the gap between the other configs). Replayed by hand without a model (`vtok_find`, `vtok_chunk ParserElement.DEFAULT_WHITE_CHARS`, `vtok_edit`), the one-line change is a `+0 lines` token patch and a one-line diff, so the edit path is sound; the cost is the agent's turns.
4. **No saving anywhere.** The cheapest way to answer these tasks is Grep and Edit. This run says nothing about long command output (`vtok run` was not part of it) or about files the agent has to read whole.
