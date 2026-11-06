# `vtok refs` vs `grep -rnw` (cross-file questions)

`python bench/refs/run.py OUT.json` (needs a release build and the project venv with `tiktoken` and `requests`). 20 names: 12 from this
repository (Rust) and 8 from `requests` (Python). For each: tokens of `grep -rnw NAME` (what an agent reads without Vtok) vs tokens of
`vtok refs NAME` (budget 800). Deterministic, no model, tiktoken o200k_base (a proxy for Claude's tokenizer).

Result (2026-11-06): `vtok refs` was smaller than grep for 20 of 20 names; median refs/grep = **0.22** over the 15 names where grep
output exceeds 150 tokens (range 0.02 - 0.80). Per name see `results.json`.

Recall: the truth for Python is Python's own `tokenize` (NAME tokens, so comments and strings are excluded); `vtok refs` found all 256 code
hits... except 7 lines, which are all usage text inside one multi-line string literal in `lib.rs` (not references). For Rust the check
is "grep hit that is not a comment line".

What the comparison does not show: grep lines carry the matching source text, `refs` carries the enclosing symbol and its line range instead,
so the agent can go straight to `vtok chunk`. Whether that saves a follow-up read depends on the task; no model was in the loop.
Caveats: names were chosen by the author in two trees, matching is by name (homonyms are merged and flagged), and `refs` only reads
Python, TypeScript/JavaScript and Rust.
