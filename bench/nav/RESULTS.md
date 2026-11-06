# Navigation benchmark: cost to view one symbol (no LLM, deterministic)

`python bench/nav/run.py OUT.json --corpus python=DIR --corpus rust=DIR --corpus typescript=DIR[,DIR] --per-bucket 25`

225 tasks (Python stdlib, Rust crates from the cargo registry, TS/JS/TSX files collected from the machine;
25 files per language and size bucket, one random symbol >= 5 lines per file, seed 1). Vtok numbers are the real
`vtok mcp` outputs. Tokens: tiktoken o200k_base (proxy for Claude's tokenizer). Raw arms have no line-number
prefixes (a real Read tool adds some, which favours the baselines). Raw file list and per-task numbers:
`results/nav-2026-11-06.json`.

Median tokens to view one symbol:

| lines in file | full read | grep + exact lines (oracle) | grep + 80-line windows | vtok index + chunk | vtok skeleton + chunk | grep + vtok chunk | vtok chunk alone |
|---|---:|---:|---:|---:|---:|---:|---:|
| Python 150-399 | 2497 | 111 | 726 | 723 | 630 | 141 | 89 |
| Python 400-999 | 5240 | 215 | 797 | 1055 | 940 | 208 | 104 |
| Python 1000+ | 13536 | 251 | 719 | 2320 | 2076 | 248 | 124 |
| Rust 150-399 | 1887 | 286 | 693 | 936 | 887 | 286 | 159 |
| Rust 400-999 | 5572 | 211 | 758 | 1781 | 1614 | 258 | 136 |
| Rust 1000+ | 13612 | 269 | 778 | 3877 | 3461 | 268 | 130 |
| TS/JS 150-399 | 2617 | 481 | 745 | 913 | 798 | 380 | 189 |
| TS/JS 400-999 | 4580 | 360 | 808 | 1138 | 1064 | 345 | 166 |
| TS/JS 1000+ | 10349 | 200 | 805 | 1751 | 1498 | 238 | 171 |
| **all (225)** | 5302 | 232 | 751 | 1494 | 1353 | 236 | 147 |

Reading (median per-task ratio, share of tasks where Vtok is cheaper):
- vs reading the whole file: index+chunk 0.26x (211/225); chunk alone 0.03x (225/225).
- vs `grep` + exact lines (an ORACLE no real agent matches: it knows where the symbol ends): chunk alone 0.80x (154/225);
  grep + vtok chunk 1.06x (96/225, a tie). Skeleton-first costs 5.5x the oracle: **do not read the skeleton when you
  already know the name**.
- vs `grep` + 80-line windows (closer to what agents do): grep + vtok chunk 0.34x (219/225); skeleton+chunk 1.53x (63/225).
- A names-only outline (`vtok symbols`, id + line range, no signatures, no alias table) is 354 tokens median vs
  1353 for the skeleton (0.35x); outline + chunk is 0.82x of grep + windows (138/225) and 0.14x of a full read, but
  2.3x the oracle (60/225).

Not measured / caveats: only "view one symbol" (no edit, no discovery by behaviour), comments and docstrings are
dropped by Vtok (the raw arms keep them), no answer-quality check (that needs model runs), tokenizer is a proxy,
files are not a random sample of any population.
