"""Vtok-TS grammar tables."""
# line-leading keyword mapping (ts word -> vtok)
KW = {
    "function": "f", "class": "C", "interface": "I", "const": "k", "let": "l", "var": "v",
    "import": "m", "return": "r", "throw": "R", "break": "B", "continue": "K",
    "try": "t", "catch": "x", "finally": "y", "switch": "S", "for": "L", "while": "w",
    "type": "Y", "export": "X", "else": "E",
}
KW_INV = {v: k for k, v in KW.items()}
KW_INV["?"] = "if"
KW_INV["e"] = "else if"
RESERVED = set(KW.values()) | {"e"}
# modifiers that may precede the keyword on the same line (kept verbatim except export)
MODS = {"default", "async", "declare", "abstract", "static", "public", "private", "protected",
        "readonly", "override", "export"}
# keywords valid alone on a line
BARE_OK = {"r", "B", "K", "E", "t", "y", "x"}
# conditions: vtok word -> (ts word, wraps condition in parens)
COND = {"?": "if", "e": "else if", "w": "while", "S": "switch"}
# lines that start a block declaration/statement: never get a trailing ';'
BLOCKISH = {"function", "class", "interface", "enum", "namespace", "module", "if", "else", "else if",
            "for", "while", "switch", "try", "catch", "finally", "case", "default"}

GRAMMAR_PROMPT = """Vtok-TS: minified TypeScript. One top-level statement per line, no comments, no optional spaces,
no ';' before '}' or at line end. Valid TypeScript otherwise (braces kept).
Names in the alias table (alias=original) are used shortened; new names are written in full.
Example:
export function val(u:string,p:string):boolean{if(!u||!p)return false;for(const x of xs)use(x);return db.q(u,p)}
"""
