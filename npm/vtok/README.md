# vtok (npm)

Compact code notation that cuts the tokens an LLM spends reading code. Native Rust core (no Python needed),
Python and TypeScript sources supported.

```js
const vtok = require('vtok');

const r = vtok.encode('typescript', source);        // { vtok, table, literals }
// ...send r.vtok (and the table) to the model, get Vtok back...
const code = vtok.decode('typescript', modelReply, r.table, r.literals);

vtok.encode('python', src, { view: 'skeleton' });    // signatures only (read-only context)
vtok.encode('python', src, { hideLiterals: 5 });     // hide long string literals behind placeholders
const s = vtok.split('python', bigFile);             // index + chunk map for big files
const c = vtok.chunk('python', bigFile, 'c3');       // full Vtok of one chunk
const out = vtok.apply('python', bigFile, { c3: editedVtok }); // splice back into the original file
```

CLI: `npx vtok encode file.py`, `npx vtok split big.ts`, `npx vtok stats src/` ... (`vtok --help`).

Build from source: `npm install && npm run build` (needs a Rust toolchain). Prebuilt binaries are published
per platform by the release workflow. License: MIT.
