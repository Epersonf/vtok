# Vtok vs graphify (AST graph, no LLM), same trees, same questions

`python bench/graph/run.py OUT.json --graphify PATH --vtok-copy DIR --requests-copy DIR` (graphify installed in its own venv from the
clone, graphs built with `graphify update` on copies of the two trees; ~10 s per tree). Tokens: tiktoken o200k_base (proxy). No model
in the loop, author-chosen questions and names, two small trees. 2026-11-06, graphify v8 from the clone.

## 1. Orientation: "where is X handled?" (18 questions, graphify budget 300, vtok find / map budget 700)

Marks: `F` = target file named, `S` = target symbol named, `-` = missed, `.` = the question has no target symbol (alias question).
graphify does not honour its budget strictly (a 300 budget printed ~450 tokens; 700 printed ~700-880).

| tree | question | graphify query | vtok find | vtok map |
|---|---|---|---|---|
| vtok | outputs stored and garbage-collected | 46 -- | 263 F- | 665 F- |
| vtok | split a big file into chunks | 454 FS | 242 FS | 665 F- |
| vtok | MCP tool dispatch | 487 FS | 264 F- | 665 F- |
| vtok | max-lines configuration order | 453 FS | 252 FS | 665 F- |
| vtok | alias assignment for names | 479 F. | 262 F. | 665 F. |
| vtok | token counting | 443 FS | 257 FS | 665 FS |
| vtok | find uses of a symbol across files | 452 FS | 221 FS | 665 FS |
| vtok | Python encoder | 475 FS | 286 -- | 665 F- |
| vtok | usage log and gain report | 459 FS | 206 FS | 665 FS |
| vtok | AGENTS.md block writer | 716 FS | 119 F- | 665 FS |
| requests | redirect handling | 398 FS | 196 FS | 638 F- |
| requests | cookie jar | 434 FS | 202 FS | 638 FS |
| requests | basic auth | 375 FS | 197 FS | 638 F- |
| requests | proxy selection | 405 FS | 94 FS | 638 FS |
| requests | connection pooling adapter | 402 FS | 194 FS | 638 FS |
| requests | decode a JSON response | 413 FS | 185 FS | 638 F- |
| requests | exception hierarchy | 371 FS | 81 FS | 638 FS |
| requests | encode the request body | 425 FS | 210 FS | 638 F- |
| | **total (tokens, files named, symbols named)** | 7687, 17/18, 16/17 | 3731, 17/18, 13/17 | 11754, 18/18, 8/17 |

Summary: vtok find names the target file as often as graphify (17/18) and the target symbol less often (13/17 vs 16/17), for **less than half the
tokens** (3731 vs 7687; 13028 with graphify at budget 700, same hits). `vtok map` names every file but few symbols (8/17) and costs the most.
Where `vtok find` misses, the question uses words that are not in the names (`garbage-collected` -> `gc`, `tool dispatch` -> `call`,
`block writer` -> `merge`), or is ambiguous between files (`Python encoder`: it returns `plugin.rs`, the target was `lib.rs`).
graphify's one total miss (`garbage-collected`, 46 tokens) is the same question: it matched the word `command` only.
The comparison is still not symmetrical: questions share words with names, which favours both keyword matchers.

Tried and dropped: appending the names of every symbol of the two best-matching files to `vtok find` (a fallback to the file outline). Tokens went from 3731 to 9551 and symbols named stayed 13/17: the misses are questions whose words are in no name, and the best-scored files were not the right ones. Keeping `find` as it is.

graphify's `GRAPH_REPORT.md` (its orientation document): 1653 tokens for the vtok tree (names 5/10 target files, 5/9 symbols), 2347 for requests (8/8, 8/8).

## 2. Who uses NAME? (10 names in `requests`)

Truth: Python's `ast`, functions whose body mentions NAME as a load or attribute (this is also name-based, so it favours name-based tools:
it counts real calls such as `self.prepare_request(...)`, and also mentions that are not calls).

| | tokens (10 names) | mean recall of calling functions | extra functions reported |
|---|---|---|---|
| `graphify explain NAME --all` | 4898 | 0.13 | 0 |
| `vtok refs NAME` | 1611 | 0.90 | 8 (module-level groups, 'code', 'request') |

graphify only lists callers it could resolve to a definition: it missed real method calls (`prepare_request`, `get_adapter`,
`resolve_redirects`), names imported from outside the tree (`urlparse`: 9 tokens, nothing) and class uses, and it found nothing for 7 of
10 names. It reported no false callers. `vtok refs` found nearly everything for about a third of the tokens, at the price of merging
homonyms and reporting some non-function enclosures. "Recall" here measures name mentions; whether the extra mentions matter depends
on the task.

## What this does not cover
graphify also offers communities, shortest paths between nodes, edge provenance (EXTRACTED / INFERRED), a cross-repo merge and, with an
LLM key, semantic extraction of docs. None of that was measured. Vtok, for its part, shortens the code and edits it in place, which
graphify does not do.
