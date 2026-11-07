# Vtok

Vtok turns source code into a compact notation so that code costs fewer tokens when it is sent to an LLM as
context. Everything is deterministic and local: the model reads and writes Vtok, and a decoder converts its
answer back into ordinary source code. No step of the round trip depends on an LLM being right, only the
generation of the answer does (and invalid output is caught and can be retried).

    source code --encode--> Vtok (+ alias table) --> LLM --> Vtok --decode--> source code

Status: **early prototype** (Python implementation). Languages: Python, TypeScript/JavaScript (`.js .mjs .cjs .jsx .tsx` via the TS grammar), Rust (Rust: native binary only, `vtok-rs` crate).
Generic tree-sitter plugins (native binary only, crate `vtok-gen`): Java, Go, C, C++, C#, Dart, CSS, HTML, SQL. One implementation driven by a per-grammar spec: comments dropped, whitespace minimal, identifiers aliased (not in HTML/CSS/SQL), line breaks kept only where the grammar needs them (Go statements, C/C++ `#` lines). `encode` always re-parses its own output and compares the tree with the original; if that fails it retries keeping the source spacing, then without aliases, and otherwise refuses the file. Files tree-sitter cannot parse are refused too. Measured on real code (tiktoken, incl. alias table): Java -48%, C++ -53%, C -31%, Go -30%, CSS -26%, C# -23% (tiny sample); HTML about 0% on text-heavy docs, so use it for markup-heavy pages only. Round trip on 369 Java, 666 Go, 6 CSS and 8 C# files: no failures; C/C++ failures are almost all files that tree-sitter itself cannot parse (macros).

## Layout (core + optional language plugins)

    packages/vtok                  core: Language ABC, plugin registry, alias planning, token counting, CLI
    packages/vtok-lang-python      Python plugin   (stdlib `ast`)
    packages/vtok-lang-typescript  TypeScript plugin (tree-sitter)
    bench/                         LLM self-validation tasks, prompt builder, checker, results

Plugins are discovered through entry points (group `vtok.languages`); install only what you need:

    pip install vtok                 # core, no language
    pip install vtok[python]         # + Python plugin   (or: pip install vtok-lang-python)
    pip install vtok[typescript]
    # development:
    pip install -e packages/vtok -e packages/vtok-lang-python -e packages/vtok-lang-typescript

To add a language, subclass `vtok.Language` (`encode`, `decode`, `analyze`, `extensions`, `grammar_prompt`;
chunking hooks are optional) and register it under `[project.entry-points."vtok.languages"]`.

## Rust core and packages

`rust/` is a Cargo workspace with the whole pipeline as one native library: `vtok-core` (tokens, alias
planning, chunking), `vtok-ts` and `vtok-py` (tree-sitter language plugins), `vtok-api` (string-based API)
and `vtok-cli` (the `vtok` executable). Two thin bindings expose exactly the same API:

| package | path | install / build |
|---|---|---|
| npm `vtok` (napi-rs addon + `vtok` bin) | `npm/vtok` | `npm install && npm run build` |
| pip `vtok` (PyO3 / maturin wheel + `vtok` script) | `pip` | `maturin develop --release` |
| native CLI | `rust` | `cargo build --release --manifest-path rust/Cargo.toml` |

Release workflows (`ci/github-workflows/`, copy them to `.github/workflows/`; pushing that folder needs a token with the `workflow` scope) build wheels and per-platform npm binaries for Linux, macOS and
Windows; they have not been run yet (they need `NPM_TOKEN` / `PYPI_API_TOKEN`). The pure-Python packages in
`packages/` are the reference implementation that the Rust core is cross-checked against (they are not
published).

Cross-checks: `python rust/difftest_ts.py <dir>` (TypeScript output byte-identical to the reference on
rxjs), `python rust/oracle_py.py <dir>...` (Python round trip checked with CPython's own `ast`; the whole
stdlib passes) and `python rust/oracle_chunks.py <dir>` (chunk edits keep the AST, docstrings included).
`cargo test --release` also checks the chunk invariants on real corpora (`VTOK_PY_CORPUS`,
`VTOK_TS_CORPUS`): unchanged chunks reproduce the file byte for byte, forced re-encoding stays equivalent.

Decoded output: TypeScript comes back minified (run your formatter); Python comes back with 4-space
statement layout but compact expressions (`a=b+c`), valid either way.

## Usage

    vtok languages                              # installed plugins
    vtok encode f.py [--table t.txt] [--hide-literals 5 --literals l.json] [--view full|skeleton|lean]
    vtok decode f.vtok --lang python --table t.txt [--literals l.json]
    vtok stats <files|dirs> [--view skeleton]   # token counts (tiktoken o200k_base)
    vtok roundtrip <files|dirs>                 # verifies the AST/tree is preserved
    vtok grammar --lang python                  # the short grammar prompt to give the LLM
    python -m pytest -q packages/*/tests

Large files: `vtok split big.py` prints an index (signatures + chunk map), `vtok chunk big.py c7` prints the
full Vtok of one chunk, and `vtok apply big.py -e c7=edited.vtok -o big.py` decodes an edited chunk and splices
it into the ORIGINAL file by line range, leaving every other chunk byte-identical.

## How it works

- **Lossless view (`full`)**: *native-tight* notation. The text is still real Python / TypeScript, so the
  model needs almost no grammar prompt (about 100 tokens): comments and docstrings removed, no optional
  whitespace, Python uses 1-space indents with single-statement bodies inlined (`if x:return 1`) and simple
  statements joined by `;`, TypeScript is one minified statement per line. A deterministic alias table
  shortens frequent names (exact break-even computed with the real tokenizer). `encode()` always verifies
  `decode(encode(x))` against `x` (AST for Python, canonical tree-sitter dump for TypeScript). Decoding is
  just parse + alias reversal (+ `ast.unparse` for Python); TypeScript output is minified, so run your
  formatter afterwards. Earlier experiments with one-letter keywords, `!`/`&`/`|` operators and removed
  braces/colons showed no token gain (frequent keywords are already single tokens) and more model errors, so
  they are kept only as `style="legacy"`.
- **`skeleton` view**: signatures only, function bodies elided. Read-only context for files the model does not edit.
- **Project table** (`plan_project`): one stable alias table for many files, paid once.
- **Chunking** (`split` / `chunk` / `apply`): files above `--max-lines` (default 150) become an index plus
  chunks cut on definition boundaries (large classes are split per method).
- **Retry loop**: `decode` raises `VtokError` with the Vtok line number, meant to be fed back to the model.

## Measurements (tiktoken o200k_base; proxy for Claude's tokenizer)

Savings versus the original source, including alias table and the one-time grammar prompt:

| scenario | Python (stdlib subset) | TypeScript (rxjs) |
|---|---|---|
| full view | -43% | -68% |
| index + 2 chunks for files > 150 lines | -69% | -69% |

Most of that comes from dropping comments and docstrings. Against the same code with comments already
removed, the notation itself saves about 11.6% (Python) and 16.4% (TypeScript). The `skeleton` view cuts
-79% (Python) / -48% (TypeScript) against comment-free code. Reading a >= 400-line file through the index plus
one average chunk costs about 27% of the raw tokens.

Honest caveats: one-character keywords gain about nothing in tokens (`function` and `return` are already one
token), which is why the default profile does not use them; the fixed grammar+example prompt (~100 tokens) makes Vtok *larger* than raw code on tiny files; savings
need real context volume.

## LLM validation

`bench/` contains 12 small edit tasks (Python and TypeScript) with hidden tests. Fresh Claude agents that
had seen nothing of the project received only the Vtok prompt and answered in Vtok; answers were decoded
locally and tested. Details and numbers in [`bench/RESULTS.md`](bench/RESULTS.md). Summary: every answer that
decoded passed its tests; early failures were grammar ambiguities, not logic errors; on a same-model comparison
Vtok matched raw code (4/4 vs 4/4) and cut output tokens by 24%. Sample sizes are small - treat as encouraging,
not conclusive.

To reproduce: `cd bench && python make_tasks.py && python prepare.py`, give each `<task>.vtok.prompt.txt` to a
fresh model, then `python check.py <task> vtok <reply-file>`.

## Outline, and compact command output

    vtok outline FILE                 # name and line range of every symbol (a few hundred tokens), imports merged
    vtok run -- cargo test            # run a command; print errors, summary, head/tail, repeated lines collapsed
    vtok output ID --grep "FAILED|panic" --context 3     # read back part of the full output (also --lines A-B, --tail N)

`vtok run` stores the whole (ANSI-stripped) output under `~/.local/state/vtok/out` (`$VTOK_STATE`; the newest 100 files for 7
days) and prints a compact view with an id. Output under ~250 tokens, or a compact view that is not smaller, is printed as is.
The command's exit code is preserved.

Works with any agent that speaks MCP; nothing is Claude-specific. The MCP server exposes `vtok_outline`, `vtok_output`, `vtok_gain`
(read-only) and sends usage guidance in its `instructions`. `vtok_run` (runs a command) is **off by default**: enable it with
`vtok mcp --allow-run`, `VTOK_MCP_RUN=1` or `vtok config set mcp-run on`. It is user-level only: a repo's `.vtok.json` cannot enable it, so a
cloned repository cannot grant itself command execution. Without it, agents run `vtok run -- ...` in their shell tool, so existing
permission rules apply. For clients that ignore MCP instructions, `vtok init --write AGENTS.md` adds (or updates) a fenced usage block. `vtok init --agent NAME` does it for the file each agent reads (claude, codex, gemini, copilot, cursor, windsurf, cline, kilocode, kimi, opencode). Hooks are installed only for Claude Code: other agents' hook protocols need an explicit "allow" to apply a rewritten command, which would skip your own approval; if your agent has a shell hook, call `vtok rewrite COMMAND` from it.

**Instruction files.** `vtok squeeze CLAUDE.md` removes whitespace, rules, bold markers and filler phrases from the prose (code blocks, inline code, URLs, headings and tables are untouched); `--write` replaces the file and keeps `CLAUDE.md.orig`.
`vtok squeeze --rewrite FILE` prints a rewrite request for your own agent, and `vtok squeeze --check ORIGINAL NEW` verifies that every code block, inline code, URL and heading survived. Measured on a dense 2.3k-token CLAUDE.md: 3% with the deterministic pass, 8% with an agent rewrite; prose-heavy files shrink more.

**Call graph.** `vtok graph build` writes `.vtok/graph.json`: functions, methods, types and documents as nodes (markdown, txt/rst/adoc/html/docx, PDFs with their headings, images with dimensions and SVG text, video/audio with size and duration where cheap, plus sidecar `.srt`/`.vtt`/`.txt` subtitles; documents link to the code symbols they name), name-based call edges, and communities (groups of related symbols; shared helpers used everywhere are left out of the grouping). Then `vtok graph explain NAME` (callers and callees), `path A B` (shortest chain), `hubs`, `communities`. The graph refreshes itself when a source file is newer; `vtok graph hook` adds a git post-commit hook that rebuilds it. `vtok graph report` writes `.vtok/GRAPH_REPORT.md` (hubs, subsystems, unexpected links between subsystems, questions to ask) and `vtok graph html` writes a self-contained interactive page `.vtok/graph.html` (force layout, search, click a node for its neighbours, toggle communities; no network, no libraries; the 700 most connected symbols). Plain values (locals, constants) are left out of the grouping and the hubs. Every edge carries a confidence tag shown by `explain` and `path` (a path is as strong as its weakest hop): EXTRACTED when the name has one definition, INFERRED when several homonyms were narrowed by the using file's imports, shared directory, same file, or a receiver that names the owning type; homonyms that cannot be narrowed get no edge. Still name-based (no type resolution); no OCR, so scanned PDFs and image contents are not read. Also available to agents as `vtok_graph` with `VTOK_MCP_ALL=1`.

**Status bar and answer style.** `vtok gain --line` prints one short line (`vtok saved 12.3k tok (41%) / 30d`) for a status bar. `vtok init --terse` prints an optional answer-style block (short answers, one-line review comments, terse commit messages). It is off by default: in our loop measurements a terse-style prompt cut output tokens by 20% but raised total cost by 7%.

**Where would it help?** `vtok discover` reads your local Claude Code transcripts, takes every shell command whose output was large, runs Vtok's compactor on the output that command really returned, and ranks command kinds by tokens saved (and whether the hook would wrap them). Read-only. The figure is an upper bound: it counts compaction of the whole output, including `cat`/`sed` file dumps that the hook does not touch.

**Native hooks for other agents.** `vtok init --hook AGENT [--global]` registers the hook (or writes the plugin file) for claude, copilot (VS Code), droid, trae, vibe, cursor, codex, gemini, antigravity, opencode, pi, omp and hermes; `vtok hook --agent NAME` speaks each agent's protocol. Cursor, Codex, Gemini CLI and Antigravity only apply a rewrite when the hook answers "allow", so for those agents only read-only and build/test commands (`git status`, `cargo test`, `pytest`, `kubectl get`, ...) are rewritten and everything that installs, pushes, deploys or deletes is left to the agent's own approval. The others (claude, copilot, droid, trae, vibe) rewrite without granting any permission. The opencode, pi/omp and hermes plugins are thin adapters over `vtok rewrite`.

**Browse.** `vtok browse URL [--max-tokens N] [--no-links]` fetches a page (http/https, 5 MB cap, public hosts only unless `--allow-private`) and prints it as light Markdown: headings, lists, tables, code, links kept; scripts, styles, navigation, footers and SVG dropped. The whole converted page stays under an id for `vtok output ID --grep ...`. Measured: a Wikipedia article 350,822 -> 27,623 tokens, the Python `json` docs 32,975 -> 7,819. MCP: `vtok_browse` (with `VTOK_MCP_ALL=1`).

**Model routing.** `vtok route "request"` scores a request (length, hard- and simple-task keywords, code blocks, files involved) and answers a tier (small / medium / large), a reasoning effort and the model for that tier (`"models": {"small": ..., "medium": ..., "large": ...}` in the user config, or `VTOK_MODEL_SMALL|MEDIUM|LARGE`, defaults haiku / sonnet / opus). `vtok proxy [--port 8787] [--upstream URL]` is a local 127.0.0.1 proxy in front of an Anthropic- or OpenAI-compatible API: a request whose model is `auto` gets the routed model (`x-vtok-route` response header), everything else, including streams, passes through untouched. Point the agent at it with its base-URL setting (`ANTHROPIC_BASE_URL=http://127.0.0.1:8787`) and pick the model name `auto`; it works with clients that accept an arbitrary model name. The router is a heuristic, not measured against task outcomes yet. MCP: `vtok_route`.

**Any agent, no hook: shims.** `vtok shims install` writes small wrapper scripts (cargo, npm, pnpm, pytest, jest, vitest, tsc, eslint, ruff, mypy, make, docker, go, dotnet, terraform and more; `vtok shims list`) into `~/.vtok/shims`. Put that directory first on `PATH` and every agent that runs shell commands (Claude Code, Codex, Gemini CLI, Cursor, Aider, Cline, a plain script) gets the compact `vtok run` view with nothing agent-specific to configure. A shim compacts only long-output tools and only when stdout is not a terminal, so you still see real output at your own prompt; the exit code is preserved, `VTOK_OFF=1` makes the shims plain passthrough, `VTOK_FORCE=1` compacts even on a terminal. Data producers (git, grep, find, ls) are deliberately not shimmed, because their output is often piped into other programs and must stay exact.

**Automatic compaction of shell output (Claude Code hook).** `vtok init --claude-hook ~/.claude/settings.json` registers a PreToolUse hook
(`vtok hook`) that rewrites known long-output commands (tests, builds, installs, `git log/diff`, recursive `grep`/`find`) to
`vtok run -- <command>` before they run, so the agent gets the compact view without asking for it. Compound lines (`a && b`, `a; b`, `a || b`) are rewritten segment by segment; a segment with a pipe, redirect or substitution, a background `&`, a heredoc and anything unrecognised pass through untouched; the hook never grants permission by itself. Agent-neutral setups can alias the same
thing with `vtok run --`, or call `vtok rewrite COMMAND` from their own hook (prints the wrapped line, exit 1 when nothing should change).

`vtok run` also has per-tool views, picked by the shape of the output: compiler and linter diagnostics (tsc, ruff, flake8, mypy, eslint) grouped by rule and message with `file:lines`, the long `git status` grouped by state, jest/vitest (failures and summary), npm/pnpm/pip installs, cargo builds (crates counted, warnings kept), terraform plans, go test and Maven/Gradle.

**RTK's declarative filters.** `vtok run` also runs the 62 command filters of [RTK](https://github.com/rtk-ai/rtk) (make, terraform/tofu plan, helm, pulumi, gcc, jq, shellcheck, yamllint, systemctl, df, ps, xcodebuild, ...; Apache-2.0, see `NOTICE`) with their own inline tests as our test suite. The same schema works in `~/.config/vtok/filters.toml` for your commands.

**Your own filters.** For a command vtok does not know, put rules in `~/.config/vtok/filters.json` (or `$VTOK_FILTERS`): `[{"match": "make deploy", "drop_contains": ["Refreshing"], "keep_contains": [], "head": 3, "tail": 10}]`. `match` is a substring of the command line, matching lines are dropped or (with `keep_contains`) kept, `head`/`tail` cap the rest. Plain substrings, no regex. The file is read from the user config only, never from a repository, so a cloned project cannot hide output from its agent.

`vtok output --grep` shows at most 40 matching lines and says how many more there are. Agents are told to skip Vtok when the plain alternative is cheaper (small files, short output).

`vtok gain [--days N] [--json]` reports what the tools saved (an upper bound versus raw output; `output` reads count as extra cost;
disable logging with `VTOK_NO_USAGE=1`). Benchmarks: `bench/nav/RESULTS.md`, `bench/run/RESULTS.md`.

## Where is it defined, who uses it

    vtok refs NAME [DIR] [--max-tokens N]

Searches a project for the identifier `NAME` and prints, one line each, the symbols that define it and the symbols that use it, with
file and line ranges you can hand to `vtok chunk` (MCP: `vtok_refs`). It matches **by name, not by type**: two functions with the
same name in different types are not told apart, and the output says so when several definitions share the name. Names in comments
and strings are not uses. Files that do not contain the text are never parsed. Benchmark: `bench/refs/RESULTS.md`.

    vtok find WORDS... [--dir DIR]    # which symbols match a question (names, parents, path, signature words)  (MCP: vtok_find)
    vtok map [DIR] [--max-tokens N]   # every file, with its most used symbols (MCP: vtok_map)

The map lists each file with its size and its most used symbols (line range, `xN` uses in other files, `~` when several files define the
name) and adds signatures for the top few, within a token budget. Honest note: it names the right file for every benchmark question but
so does a plain file listing at a fraction of the cost; the map pays when file names are not informative (several `lib.rs`) or when you
want the symbol too. See `bench/map/RESULTS.md`. `vtok find` ranks symbols by how well their name words match the words of a question (it matches names, not meaning or comments). Head to head with graphify: `bench/graph/RESULTS.md`.

## Configuration

The size above which a file is split into chunks (default 150 lines) is configurable:

    vtok config                       # show the effective value and where it comes from
    vtok config set max-lines 300     # for your user (~/.config/vtok/config.json, or $VTOK_CONFIG)
    vtok config set max-lines 80 --local   # for this repo (.vtok.json, found by walking up from the file)
    vtok config unset max-lines [--local]

Order of precedence: `--max-lines` / the MCP `max_lines` argument, `VTOK_MAX_LINES`, nearest `.vtok.json`, the user file, 150.

## Known limits

- Docstrings and comments are dropped in the Vtok view; `apply` preserves them for untouched chunks, and
  Python docstrings are re-attached inside edited chunks (comments inside an edited chunk are lost).
- TypeScript/JavaScript: semicolon-less (ASI) sources are supported; JSX/TSX supported (JSX text whitespace is normalized per JSX semantics), no Flow; multi-line template literals become `
` escapes (error if tagged); decoded output is
  minified (format it afterwards).
- Rust: tree-sitter-rust gaps make ~0.6% of crates.io files unparseable (attributes in struct patterns, `where` on unit structs, `const default`, exotic macro_rules tokens); vtok refuses them.
- Python: decoded output goes through `ast.unparse` (canonical formatting).
- Rust (`rust/crates/vtok-rs`, tree-sitter-rust 0.23): every token is kept (`;` and `,` included), so the
  notation only drops comments and optional whitespace; decoded output is minified (run `rustfmt`). Names in
  `use`, attributes, macro names, format-string captures, `pub` items and trait members are never aliased.
  Grammar gaps (e.g. a variable named `raw` after `&`, `unsafe extern` blocks) are rejected with a syntax error.

## Extra ideas, measured (tiktoken o200k_base, share of comment-free native-tight tokens)

| idea | Python | TypeScript | status |
|---|---|---|---|
| Aliases that cost one token (single letters, then one-token words) | -3.9% | -2.6% | **adopted** |
| Hide string literals >= 5 tokens (`--hide-literals 5`, decoder restores from a local file) | -6.2% | -0.5% | **implemented**, opt-in (the model cannot see the text; TS gain is small because most strings are import paths and type literals, which are never hidden) |
| `lean` view: no type annotations, imports as names only (`--view lean`, read-only) | -0.1% | -28% | **implemented** (lossy, for files the model will not edit) |
| Short names for locals | already covered by aliasing | | |
| Tab instead of spaces for indentation | +0.6% (worse) | n/a | rejected |
| Drop `self.` / `this.` (decoder re-inserts) | -0.8% | -0.2% | rejected: tiny gain, high ambiguity risk |
| Drop decorators in skeleton | -0.3% | 0% | rejected: negligible |
| Grouped imports line | n/a | at most -6.8% (upper bound; loses `as`/default/type forms) | not implemented; a lossless form would gain far less |
| Boilerplate macros (trivial getters, `__init__`) | not measured | | not implemented; needs per-framework pattern matching |

## License

MIT, see [LICENSE](LICENSE).
