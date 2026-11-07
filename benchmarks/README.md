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
- Caveman's value is the terse response style (model side), which this does not measure; its `compress` engine barely
  changes tool output.
- Small corpus, one machine, no repeat runs, no agent in the loop. It says nothing about task success.

Graph, same 115 Rust files: Graphify `update` 4.8 s, 1,296 nodes, 3,351 edges, 1.26 MB, report 2,935 tokens;
`vtok graph build` 0.33 s, 1,153 nodes, 2,638 edges, 110 KB, report 794 tokens. Edge precision is not measured
(name-based resolution gives false positives, e.g. a call to a generic `text`).
