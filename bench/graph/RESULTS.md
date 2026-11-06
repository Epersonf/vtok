# Vtok vs graphify (AST graph, no LLM), same trees, same questions

`python bench/graph/run.py OUT.json --graphify PATH --vtok-copy DIR --requests-copy DIR` (graphify installed in its own venv from the
clone, graphs built with `graphify update` on copies of the two trees; ~10 s per tree). Tokens: tiktoken o200k_base (proxy). No model
in the loop, author-chosen questions and names, two small trees. 2026-11-06, graphify v8 from the clone.

## 1. Orientation: "where is X handled?" (18 questions)

| | tokens | target file named | target symbol named |
|---|---|---|---|
| `graphify query "<question>" --budget 700` (vtok tree, 10 q) | median 849 | 9/10 | 8/9 |
| same (requests, 8 q) | median 699 | 8/8 | 8/8 |
| graphify `GRAPH_REPORT.md` (vtok / requests) | 1653 / 2347 | 5/10, 8/8 | 5/9, 8/8 |
| `vtok map` @700 (bench/map) | ~650 | 10/10, 8/8 | 4/9, 4/8 |
| `vtok map` @1500 | ~1500 | 10/10, 8/8 | 6/9, 8/8 |

graphify is **better at orientation** here: for roughly the same tokens its question-driven query names the target symbol far more often
than the question-independent map. The comparison is not symmetrical: graphify uses the question's words (matched against node names), the
map ignores the question. Vtok has no equivalent of a question-driven symbol search, and that is the gap this exposes. (Caveat: my
questions share words with the target names, which favours any keyword matcher.)

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
