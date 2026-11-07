# Benchmarks

Compares vtok with RTK, Caveman's engine and Graphify on real commands. Nothing here runs in CI.

```bash
python fixtures.py /tmp/fx            # failing sample projects (needs cargo, python+pytest/mypy/ruff, npm, go)
FIXTURES=/tmp/fx RTK_BIN=.../rtk VTOK_BIN=.../vtok CAVEMAN_BIN=.../caveman-engine python bench.py
```

Tokens are `(chars + 3) / 4`, the metric RTK's own benchmark uses. `keep N/M` counts key strings (failing test names,
error codes) that survive in each tool's output. It is a crude retention check, not a quality score.

## Results (one Windows machine, 19 scenarios, vtok 2c4fc3b, RTK 0.49.0)

Command output, raw 36,076 tokens: RTK 25,243 (-31%), vtok 11,200 (-69%), Caveman `compress` 36,076 (0%). The git
scenarios use fixed revisions and the `cat`/`grep` scenarios a pinned file (`fixtures.py` writes it from git history), so
the numbers do not move with every commit. `keep` counts a scenario's key strings in the output (crude).

| scenario | raw | RTK | vtok | note |
|---|---:|---:|---:|---|
| git diff (3f1264c..e27cbfc) | 10,603 | 7,562 | 5,978 | see the Diffs section: parity on retention |
| git log -30 | 1,893 | 977 | 567 | |
| git log --stat -5 | 898 | 898 | 233 | |
| ls -la | 170 | 56 | 47 | |
| grep -n | 692 | 367 | 274 | RTK loses the `fn compact_cmd` line |
| cat of a 727-line source file | 9,861 | 9,861 | 2,599 | vtok elides the middle (it lost `fn compact_cmd`); use `vtok_read` to read code |
| cargo build (errors) | 395 | 352 | 71 | |
| cargo test (3 fail) | 403 | 156 | 105 | |
| cargo test (all pass) | 4,789 | 11 | 12 | RTK 1 token smaller (vtok keeps the warning count) |
| pytest (3 fail, same message) | 328 | 104 | 55 | |
| pytest (11 fail, mixed errors) | 1,140 | 419 | 301 | RTK drops one failing test |
| mypy | 64 | 64 | 64 | below the 250-token pass-through threshold |
| ruff | 341 | 138 | 105 | |
| tsc (34 identical errors) | 653 | 672 | 57 | best case for grouping |
| tsc (47 mixed errors) | 911 | 950 | 301 | realistic: mostly distinct messages |
| eslint | 2,568 | 2,568 | 262 | RTK passes it through |
| vitest (2 fail) | 328 | 37 | 59 | RTK is smaller but keeps 0/2 failing test names (its parser fails on this vitest, 5.1.0, and it prints a pass-through notice); vtok keeps both names with expected vs received and the location (was 134) |
| go test (2 fail) | 35 | 51 | 35 | |

Where RTK is ahead: a green `cargo test` by one token, and `vitest` in size (at the cost of the failing test names).
Nothing else in this set. Every view was also audited for silently dropped error classes: with 70 distinct rules or
codes fed to each of 16 lint/compiler views, 10 of them lost classes before (oxlint, biome, rubocop, sqlfluff, phpstan,
make/gcc, dotnet, go build, eslint) and none do now (`bench/run/audit_retention.py`, `src/t_retention.rs`).

Caveats, read before quoting any number:

- vtok's totals include views that cut content (`cat`, `grep`, large `git diff`). The full output is stored under an id
  and `vtok output ID` returns any part, but the agent sees less on the first read.
- Caveman's value is the terse response style (model side), which `bench.py` does not measure (see "Answer style"
  below); its `compress` engine does not change tool output.
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

## Language plugins (`gen_eval.py`, `lossless_check.py`)

`python benchmarks/gen_eval.py css=SRC_ROOT sql=MIGRATIONS c=@files.txt ...` (or `VTOK_EVAL_<LANG>`, see the docstring)
runs the verified encode on real files of the generic plugins (css html sql java cpp cs go c dart) and prints per language
the files, the refusals, the mean saving of each view (full / skeleton / lean), the median encode time and sample outline
lines. `lossless_check.py LANG FILELIST` is an independent check that does not use the plugin's own proof: it encodes and
decodes each file with the CLI and compares original and decoded code with its own tokeniser (comments removed, string
literals exact, words whole, spacing kept only where it separates tokens that would merge). Corpora here are real code from
other projects on one machine; the numbers show relative change, not a guarantee for your code.

Refusals (a refused file gets no compaction) before and after the verbatim-region work, and mean saving after, in %
(vtok's own token estimate, alias table included):

| language | files | refused before | refused after | saving full / skeleton / lean |
|---|---:|---|---|---|
| SQL | 12 | 12 (100%) | 0 | 25.2 / 75.3 / 26.4 |
| C | 300 | 159 (53%) | 0 | 27.2 / 87.1 / 87.1 |
| C++ | 19 | 5 (26%) | 0 | 42.3 / 81.0 / 81.0 |
| HTML | 108 | 10 (9%) | 0 | 3.6 / 81.5 / 20.9 |
| CSS | 5 | 0 | 0 | 35.0 / 82.2 / 35.6 |
| Java | 300 | 0 | 0 | 49.1 / 83.0 / 83.0 |
| Go | 300 | 0 | 0 | 30.6 / 86.9 / 86.9 |

C# (Unity projects) and Dart (Flutter SDK) were also swept, with no refusals. The independent comparison found no
difference on 944 files (SQL 12, CSS 5, C 300, C++ 19, HTML 108, Java 100, Go 300, C# 100). On 6 of 100 Flutter Dart
files my comparer flags a difference that comes from its own lexing (it reads `@override@protected` as one word and does
not parse `${...}` interpolation); the Dart plugin was not changed by this work, and I did not rule the cause out beyond
that. The skeleton and lean views of CSS, HTML and SQL used to equal the full view; now CSS skeleton is the selector and
@-rule tree with declaration counts, HTML skeleton the element tree with id, class and href/src/name/type attributes, and
SQL skeleton one head per statement. Known: `vtok_edit` is rejected on a file that contains unparsed regions; reading and
outline work. The lean view of the code languages is still the same as skeleton.
