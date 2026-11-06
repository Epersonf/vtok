"""Vtok grammar tables (M0). Single source of truth for keyword mapping."""

# statement-leading keywords: python -> vtok
STMT = {
    "class": "C", "def": "f", "return": "r", "if": "?", "elif": "e", "else": "E",
    "for": "L", "while": "w", "import": "m", "from": "M", "try": "t",
    "except": "x", "finally": "y", "with": "W", "raise": "R", "pass": "p",
    "break": "B", "continue": "K", "assert": "A",
}
STMT_INV = {v: k for k, v in STMT.items()}
# headers: line ends with ':' in python (colon dropped in vtok)
HEADERS = {"C", "f", "?", "e", "E", "L", "w", "t", "x", "y", "W"}
# simple statements that may be inlined as single-line block body
INLINE_BODY = {"r", "p", "B", "K", "R"}
# expression-level tokens
EXPR = {"not": "!", "and": "&", "or": "|", "True": "T", "False": "F", "None": "N"}
EXPR_INV = {v: k for k, v in EXPR.items()}
TYPES = {"bool": "b", "str": "s", "int": "i"}
TYPES_INV = {v: k for k, v in TYPES.items()}
# names that cannot be used bare as Name tokens (collide with keywords/literals)
RESERVED = set(STMT.values()) | {"T", "F", "N"}

GRAMMAR_PROMPT = """Vtok: Python with minimal whitespace. Valid Python otherwise: 1-space indent, no spaces around
operators, a header may take one simple statement inline (`if x:return 1`), simple statements joined by ';'.
Names in the alias table (alias=original) are used shortened; new names are written in full.
Example:
class AuthMgr:
 def val(u,p)->bool:
  if not u or not p:return False
  return db.q(u,p)
"""

# bitwise operators are doubled so single | & stay free for logical or/and
BITWISE = {"|": "||", "&": "&&", "|=": "||=", "&=": "&&="}
BITWISE_INV = {v: k for k, v in BITWISE.items()}
