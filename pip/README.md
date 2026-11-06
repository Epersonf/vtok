# vtok (pip)

Compact code notation that cuts the tokens an LLM spends reading code. Native Rust core (no Python parser
needed), Python and TypeScript sources supported. Same core as the npm package.

```python
import vtok

r = vtok.encode("python", source)                 # Encoded(vtok, table, literals)
# ...send r.vtok to the model, get Vtok back...
code = vtok.decode("python", model_reply, r.table, r.literals)

vtok.encode("typescript", src, view="skeleton")   # signatures only (read-only context)
vtok.encode("python", src, hide_literals=5)       # hide long string literals behind placeholders
s = vtok.split("python", big_file)                # index + chunk map for big files
c = vtok.chunk("python", big_file, "c3")          # full Vtok of one chunk
out = vtok.apply("python", big_file, {"c3": edited})   # splice back into the original file
```

CLI: `vtok encode file.py`, `vtok split big.ts`, `vtok stats src/` (see `vtok` with no arguments).

Build from source: `pip install maturin && maturin develop --release` (needs a Rust toolchain).
License: MIT.
