class VtokError(Exception):
    """Malformed Vtok or failed verification. `line` (1-based, Vtok text) feeds the LLM retry loop."""
    def __init__(self, msg, line=None):
        self.line = line
        super().__init__(f"line {line}: {msg}" if line else msg)
