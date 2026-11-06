# `vtok map`: does the overview name what an agent should open?

`python bench/map/run.py OUT.json`. 18 author-chosen questions ("where is X handled?"): 10 in this repository, 8 in `requests`. A
question counts as answered when the map names the target FILE (and, separately, the target SYMBOL) within the given budget. No model:
this measures visibility, not whether an agent would choose correctly. Tokens: tiktoken o200k_base (proxy).

| tree | file listing | all outlines | map@300 | map@700 | map@1500 |
|---|---|---|---|---|---|
| vtok (29 files): tokens | 258 | 3522 | 289 | 666 | 1473 |
| vtok: target file named | 10/10 | 10/10 | 10/10 | 10/10 | 10/10 |
| vtok: target symbol named | - | 9/9 | 0/9 | 4/9 | 6/9 |
| requests (19 files): tokens | 64 | 3779 | 171 | 638 | 1499 |
| requests: target file named | 8/8 | 8/8 | 8/8 | 8/8 | 8/8 |
| requests: target symbol named | - | 8/8 | 0/8 | 4/8 | 8/8 |

Reading, honestly: the map always names the right file, but so does a plain file listing at 64-258 tokens. What the map adds is symbols:
about half of the target symbols at ~650 tokens, most at ~1500. Listing + `vtok outline` of the guessed file costs roughly 250-450 tokens in both
trees and names every symbol of that file, so the map is **not cheaper** when file names already point to the right file. It is
worth using when names do not (four `lib.rs`, `plugin.rs` per language crate) or when you want a quick look at which symbols the
rest of the project depends on. A first design ranked files by use count and showed only the top files; it named the target file in 3/10
and 5/8 at 700 tokens, so it was replaced by "every file gets a line, symbols are added round by round".

Ranking is by name, not by type: functions count calls (`name(`), methods count `.name` accesses, other names count plain mentions, and
uses inside files that define the same name are ignored. Names shared by several files are marked `~`. Caveats: author-chosen questions,
two trees, no model in the loop.
