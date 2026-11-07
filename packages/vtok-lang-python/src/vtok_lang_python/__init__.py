"""Vtok language plugin: Python (stdlib ast).

`encode` turns source into notation (`views` rewrites the tree first, `aliases` renames, `native` and `legacy` lay it out);
`decode` turns it back; `lexer` and `grammar` hold the token and keyword tables; `chunks` serves big files in pieces;
`plugin` is the Language that vtok discovers."""
from .plugin import Python
from .encode import encode
from .decode import decode
