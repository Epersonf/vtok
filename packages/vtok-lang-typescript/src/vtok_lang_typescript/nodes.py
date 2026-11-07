"""Node kinds and word lists of the TypeScript grammar, and the text of a node."""
IDENT = {"identifier", "type_identifier", "property_identifier"}
LIKE_IDENT = IDENT | {"property_identifier", "shorthand_property_identifier",
                      "shorthand_property_identifier_pattern", "statement_identifier",
                      "private_property_identifier"}
SKIP = {"comment", "empty_statement"}
JS_WORDS = {"abstract", "any", "as", "async", "await", "boolean", "break", "case", "catch", "class", "const",
            "continue", "debugger", "declare", "default", "delete", "do", "else", "enum", "export", "extends",
            "false", "finally", "for", "from", "function", "get", "if", "implements", "import", "in",
            "instanceof", "interface", "is", "keyof", "let", "module", "namespace", "never", "new", "null",
            "number", "object", "of", "override", "package", "private", "protected", "public", "readonly",
            "return", "satisfies", "set", "static", "string", "super", "switch", "symbol", "this", "throw",
            "true", "try", "type", "typeof", "undefined", "unknown", "var", "void", "while", "with", "yield"}

def text(n): return n.text.decode()
