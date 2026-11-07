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

## Graph accuracy (`graph_eval.py`)

Edge accuracy of `vtok graph` and Graphify against rust-analyzer's SCIP index (a compiler-grade ground truth). An edge
A -> B is true when the code inside A references B; its kind follows B (function/method = call, type = type use,
const/static = reference). Prerequisites: `rustup component add rust-analyzer`, Graphify on PATH (or `GRAPHIFY`), a built
vtok. Nothing is downloaded. Run `python benchmarks/graph_eval.py` for this repository's `rust/` workspace; point
`EVAL_SRC`, `EVAL_SUB` and `EVAL_EXTRA` at any other Cargo workspace (see the script's docstring).

Non-test code, precision / recall in %. Edges that touch files rust-analyzer did not index (code behind a `cfg` it did not
enable, such as `#[cfg(unix)]` on Windows or an optional feature) cannot be judged and are left out of precision for BOTH
tools (`EVAL_UNMAPPED=fp` also counts unmapped items inside indexed files as wrong).

| code | role | vtok calls | Graphify calls | vtok type uses | Graphify type uses |
|---|---|---|---|---|---|
| vtok's own tree | tuned on | 99.9 / 95.4 | 98.6 / 81.5 | 100 / 90.1 | 99.4 / 47.8 |
| RTK `src/` (3,426 definitions) | scores calibrated here | 99.3 / 94.8 | 98.7 / 82.4 | 99.9 / 97.0 | 99.8 / 50.3 |
| indexmap 2.14.2 | held out | 98.0 / 56.5 | 67.2 / 40.2 | 100 / 69.2 | 90.4 / 32.8 |
| aho-corasick 1.1.5 | held out | 92.0 / 54.2 | 75.1 / 34.1 | 98.8 / 95.6 | 98.9 / 50.1 |

What this says: precision is high and its classes are calibrated on code the scores were never fitted to (EXTRACTED calls
99.6-100% precise on both held-out crates; the 0.8 bucket 81-86%). Recall of calls drops from about 95% to about 55% on
the two library crates: calls on values whose type vtok cannot see (generics, trait-heavy code) are missed. Graphify is
lower on precision and recall on every held-out figure but one (type-use precision on aho-corasick, a tie). The first
version of this work reported 100% call precision on vtok's own tree and 81% on RTK (before the fixes below); an RTK
false-positive analysis then fixed the general causes (the manifest above a graphed `src/` was not found, trait dispatch
on unknown receivers, variant patterns shadowing functions). Measured on the same RTK code: vtok graph 1.2 MB, 3.7-4.4 s,
report about 920 tokens; Graphify 9.1 MB, 11-14 s, report about 24,600 tokens. On vtok's own tree: 135 KB and 1.1 s
against 1.15 MB. Only Rust has a ground truth; the other languages' resolution is covered by fixture tests, not measured.
Known: nested items (a `const` inside a function) have no node, so references to them are credited to the enclosing
function; `windows-sys`-style generated binding crates take about 17 s and write a 20 MB graph.

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

## Diffs (`diff_eval.py`)

`python benchmarks/diff_eval.py [--commits N] [--repo PATH] [--rows]` (env `RTK_BIN`, `VTOK_OLD`, `VTOK_NEW`) runs
`git show <sha>` and `git diff <sha>~1 <sha>` for real commits through raw git, RTK and vtok and compares tokens,
changed-line retention (share of added/removed lines that appear in the output, a 60-character prefix counts for a
truncated line) and file coverage, bucketed by raw size. The metric definitions are in the script's docstring.

74 rows from 37 non-merge commits of this repository (RTK's checkout is a shallow clone, so none of its commits are
included), retention as median per row / pooled over all changed lines:

| bucket | raw tokens | RTK tokens | RTK retention | vtok tokens | vtok retention | previous vtok retention |
|---|---:|---:|---|---:|---|---|
| small (7) | 2,165 | 2,015 | 100 / 100 | 1,634 | 100 / 100 | 100 / 100 |
| medium (25) | 52,279 | 43,584 | 100 / 79.9 | 37,292 | 100 / 99.1 | 32.3 / 27.3 |
| large (42) | 735,602 | 239,558 | 57.5 / 35.8 | 230,982 | 66.9 / 36.7 | 6.0 / 6.0 |
| all (74) | 790,046 | 285,157 | 80.4 / 37.6 | 269,908 | 93.9 / 39.2 | 15.9 / 6.9 |

Spot checks I ran on the merged build (same retention rule, tokens `(chars + 3) / 4`):

| diff | raw | RTK tokens / kept | vtok tokens / kept |
|---|---:|---|---|
| 3f1264c..e27cbfc (510 changed lines) | 10,603 | 7,562 / 79.6% | 5,983 / 74.7% |
| commit 57c4ad1 (342) | 7,446 | 7,121 / 100% | 5,420 / 96.5% |
| commit f766b4c (24) | 852 | 792 / 100% | 507 / 100% |
| commit 3f1264c, merge-sized (1,579) | 27,604 | 5,740 / 37.0% | 5,947 / 34.4% |
| commit f77a07d, merge-sized (1,630) | 27,949 | 5,316 / 33.7% | 5,951 / 33.2% |

Read it as parity: the earlier view kept 4% of the changed lines; now small and mid-size diffs are kept almost whole at
20-35% fewer tokens than RTK, while on commits with more than 1,500 changed lines vtok prints slightly more tokens than
RTK for slightly less retention. The budget is 2,500 + raw/2 tokens (at most 8,000), far above the 558 tokens the old
view printed, which is the price of keeping the lines.
