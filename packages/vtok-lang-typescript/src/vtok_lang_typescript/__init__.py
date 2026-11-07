"""Vtok language plugin: TypeScript (tree-sitter).

`grammar`, `parse` and `canon` define the notation and read the tree; `encode` turns source into notation, with `nodes`,
`emit`, `emitter`, `analysis` and `literals` as its parts; `decode` turns it back; `chunks` serves big files in pieces;
`plugin` is the Language that vtok discovers."""
from .plugin import TypeScript
from .encode import encode
from .decode import decode
