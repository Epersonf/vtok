"""Token counting. tiktoken (o200k_base) if available, else rough fallback."""
import re
try:
    import tiktoken
    _enc = tiktoken.get_encoding("o200k_base")
    def ntok(text: str) -> int:
        return len(_enc.encode(text))
    BACKEND = "tiktoken:o200k_base"
except Exception:  # pragma: no cover
    def ntok(text: str) -> int:
        return len(re.findall(r"[A-Za-z]+|\d+|[^\sA-Za-z\d]", text))
    BACKEND = "fallback-regex"
