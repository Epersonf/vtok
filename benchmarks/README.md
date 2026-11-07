# Benchmarks

Compares vtok with RTK, Caveman's engine and Graphify on real commands. Nothing here runs in CI.

```bash
python fixtures.py /tmp/fx            # failing sample projects (needs cargo, python+pytest/mypy/ruff, npm, go)
FIXTURES=/tmp/fx RTK_BIN=.../rtk VTOK_BIN=.../vtok CAVEMAN_BIN=.../caveman-engine python bench.py
```

Tokens are `(chars + 3) / 4`, the metric RTK's own benchmark uses. `keep N/M` counts key strings (failing test names,
error codes) that survive in each tool's output. It is a crude retention check, not a quality score.

## Results (one Windows machine, 16 scenarios, vtok `57c4ad1`, RTK 0.49.0)

Command output, raw 57,504 tokens: RTK 22,500 (-61%), vtok 6,329 (-89%), Caveman `compress` 57,448 (-1%).

| scenario | raw | RTK | vtok | note |
|---|---:|---:|---:|---|
| cargo build (errors) | 395 | 352 | 74 | both keep the error codes |
| cargo test (3 fail) | 403 | 155 | 155 | tie |
| cargo test (all pass) | 3052 | 11 | 124 | RTK better |
| pytest (3 fail) | 328 | 104 | 169 | RTK better, both keep the names |
| ruff | 341 | 138 | 108 | |
| tsc | 653 | 672 | 594 | neither helps |
| eslint | 2568 | 2568 | 478 | RTK passes it through |
| vitest (2 fail) | 332 | 37 | 138 | RTK drops the failing test names (0/2); vtok keeps them |
| git diff HEAD~3 | 35772 | 5945 | 558 | vtok shows only each file's start; RTK keeps hunks |
| cat large source file | 9861 | 9861 | 2604 | vtok elides the middle |

Caveats, read before quoting any number:

- vtok's totals include views that cut content (`git diff`, `cat`, `grep`). The full output is stored under an id and
  `vtok output ID` returns any part, but the agent sees less on the first read. RTK keeps more there.
- Caveman's value is the terse response style (model side), which `bench.py` does not measure (see "Answer style"
  below); its `compress` engine barely changes tool output.
- Small corpus, one machine, no repeat runs, no agent in the loop. It says nothing about task success.

Graph, same 115 Rust files: Graphify `update` 4.8 s, 1,296 nodes, 3,351 edges, 1.26 MB, report 2,935 tokens;
`vtok graph build` 0.33 s, 1,153 nodes, 2,638 edges, 110 KB, report 794 tokens. Edge precision is not measured
(name-based resolution gives false positives, e.g. a call to a generic `text`).

## Answer style (`style_eval.py`)

Measures the response-style note (`vtok init --terse`) against Caveman's skill with Caveman's own method: the same
prompts go through `claude -p --output-format json` under four arms (no system prompt, a plain "Answer concisely."
control, control + Caveman's `SKILL.md`, control + the vtok note) and the reported output tokens and a regex fidelity
check are compared. Prompts are Caveman's English and Portuguese lists plus 12 vtok dev-task cases, split into a DEV set
(used while tuning) and a HELD-OUT set (never looked at while tuning).

Caveman's isolation flags alone still let `~/.claude/CLAUDE.md` and the parent session's environment into every arm on
current Claude Code, so `style_claude.py` also cleans the child environment, sets `CLAUDE_CODE_DISABLE_CLAUDE_MDS=1`
and turns attribution off. One reminder remains in every arm (the account e-mail from the OAuth login); it cannot be
removed without an API key. The committed evidence is `style_snapshot.json` (per-call token counts and check results,
no prompts or answers); the raw call cache `style_cache/` stays local and is git-ignored.

```bash
# regenerate the tables from the committed snapshot, no Claude calls, no quota
python style_eval.py --report-only --set held --arms control,caveman,vtok --model claude-sonnet-5-5 --reps 2
# new calls spend quota: --max-new N caps them; needs CAVEMAN_DIR and VTOK_BIN
```

Median output-token change against the control (held-out prompts; the vtok note is +283 input tokens per call, Caveman's
skill +1716):

| model | Caveman skill | vtok note | fidelity failures (vtok / Caveman) |
|---|---:|---:|---|
| claude-sonnet-5-5 (N=2) | -14% | -36% | 0 / 0 |
| claude-haiku-5-5 | +19% | -41% | 0 / 1 |

On Haiku the long skill roughly doubles thinking tokens, so billed output goes up even though the visible text is 17-25%
shorter. These are single-turn runs on Haiku and Sonnet, not Caveman's published Opus snapshot, and the regexes check
that facts survive, not answer quality. Net of the note's own input cost the vtok note is roughly break-even per call in
raw tokens and a gain when output is priced above input.
