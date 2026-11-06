# `vtok run` benchmark (no LLM)

`python bench/run/run.py OUT.json` - runs real commands (some on fixtures built by the script), compares tokens of the raw
output with what `vtok run` shows (budget 700 tokens), and checks that the KEY FACTS (names of failing tests, error
messages, summary lines) are still visible and that `vtok output ID --grep FACT` recovers them from the stored output.

| case | raw tokens | shown | shown/raw | key facts visible | recoverable |
|---|---:|---:|---:|---:|---:|
| cargo test (passing, 3 crates) | 1042 | 315 | 0.30 | 1/1 | 1/1 |
| cargo test (1 failing of 31) | 475 | 274 | 0.58 | 3/3 | 3/3 |
| cargo build (11 compile errors) | 1153 | 644 | 0.56 | 3/3 | 3/3 |
| pytest -v (failing, 63 tests) | 1403 | 502 | 0.36 | 4/4 | 4/4 |
| pytest -v (regex package, passing) | 3854 | 743 | 0.19 | 1/1 | 1/1 |
| unittest -v (stdlib test_json) | 127 | 127 | 1.00 (short: left as is) | 1/1 | - |
| log: 4000 lines, 1 buried error (synthetic) | 112095 | 221 | 0.00 | 3/3 | 3/3 |
| git log --stat -n 60 | 9430 | 526 | 0.06 | n/a | - |
| pip list | 249 | 249 | 1.00 (short: left as is) | n/a | - |
| find site-packages -name '*.py' | 29630 | 731 | 0.03 | n/a | - |
| grep -rn 'def ' (regex pkg) | 18430 | 666 | 0.04 | n/a | - |

Median shown/raw over the 9 cases with more than 250 raw tokens: 0.19. Totals are dominated by the synthetic log and the
listings, so read the rows, not the sum.

What this does NOT show:
- The facts were chosen by the author, and the compressor was tuned on these same cases (two bugs found by this
  benchmark: `0 failed` scored as an error; `ok` counted as a summary word). Expect worse on commands I did not try.
- Listings (git log, find, grep) are cut to head/tail/hits: the rest is only reachable through `vtok output`, which costs
  a second call when the answer was in the middle. Whether that second call pays off was not measured.
- No model was asked to solve anything from the compact view; "key facts visible" is a proxy for usefulness.
- Tokenizer is a proxy (tiktoken o200k_base).

## Head to head with rtk 0.49.0 (same 11 cases, `--rtk PATH`, 2026-11-06)

rtk was built from source and run with its own subcommands (`rtk cargo test`, `rtk pytest`, `rtk find`, ...). Tokens shown to the agent, tiktoken o200k proxy. Facts are author-chosen strings; rtk words some results differently, so its fact column is a lower bound. Vtok was tuned on these same cases after the first round, so treat the Vtok column as optimistic. The `unittest` case was replaced by a real fixture (the old one imported a stdlib test package that is not installed).

| case | raw | vtok round 1 | vtok now | rtk |
|---|---|---|---|---|
| cargo test (passing) | 1075 | 436 | 21 | 16 |
| cargo test (1 failing) | 472 | 276 | 105 | 132 |
| cargo build (11 errors) | 1150 | 642 | 368 | 1120 |
| pytest -v (63 failing) | 1400 | 497 | 134 | 159 |
| pytest -v (regex, passing) | 3367 | 739 | 29 | 13 |
| unittest -v (40 pass, 2 fail) | 758 | - | 102 | 54 |
| log, 4000 lines | 112095 | 221 | 122 | 83 |
| git log --stat | 10899 | 591 | 250 | 10899 (passthrough) |
| pip list | 249 | 249 | 221 | 218 |
| find | 29630 | 731 | 185 | 202 |
| grep -rn | 18430 | 667 | 133 | 1536 |

Median shown/raw over the larger cases: 0.19 -> 0.02.

Vtok is smaller on cargo test (failing), cargo build, pytest (failing), git log, find and grep, and kept every fact the benchmark checks. rtk is still smaller on passing runs (cargo test 16 vs 21, pytest 13 vs 29), the unittest fixture (54 vs 102: rtk drops test names and locations, which Vtok keeps; it matched 1 of 3 facts), the log (83 vs 122: rtk drops the traceback frames and the shutdown line) and, by 3 tokens, `pip list`.
