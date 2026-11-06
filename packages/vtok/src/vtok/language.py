from abc import ABC, abstractmethod
from dataclasses import dataclass

@dataclass
class Analysis:
    counts: dict      # name -> occurrences (aliasable)
    excluded: set     # names that must never be aliased
    avoid: set        # every identifier-like text (aliases must not collide)
    reserved: set     # names that collide with Vtok tokens (always aliased)

@dataclass
class Unit:
    name: str
    kind: str
    start: int            # 1-based first line (incl. decorators)
    end: int              # 1-based last line (inclusive)
    member: bool = False  # child of a split class
    parent: str | None = None
    indent: int = 0

@dataclass
class Encoded:
    vtok: str
    table: dict  # original -> alias
    literals: dict = None  # placeholder -> hidden literal (kept locally, never shown to the model)

class Language(ABC):
    """Contract every language plugin implements."""
    name: str = ""
    extensions: tuple = ()
    grammar_prompt: str = ""   # short rules + example shown to the LLM

    @abstractmethod
    def encode(self, source: str, *, alias: bool = True, verify: bool = True, table: dict | None = None, view: str = "full", hide_literals: int = 0) -> Encoded:
        """table: fixed {original: alias} (project-level); overrides per-file planning.
        view: 'full' (lossless), 'skeleton' (signatures only; read-only) or 'lean' (no type annotations / import paths; read-only).
        hide_literals: replace string literals of at least this many tokens by placeholders (0 = off); see Encoded.literals."""

    @abstractmethod
    def analyze(self, source: str) -> Analysis: ...

    @abstractmethod
    def decode(self, vtok: str, table: dict | None = None, literals: dict | None = None) -> str:
        """table: {alias: original}. Raise VtokError (with line) on malformed input."""

    # ---- chunking hooks (optional; needed for `vtok split/chunk/apply`) ----
    def units(self, source: str, max_lines: int) -> list:
        raise NotImplementedError(f"{self.name} plugin does not support chunking")

    def encode_chunk(self, text: str, table: dict, member: bool) -> str:
        raise NotImplementedError

    def decode_chunk(self, vtok: str, table: dict, member: bool, indent: int, original: str | None = None) -> str:
        raise NotImplementedError

    def equivalent(self, a: str, b: str, member: bool) -> bool:
        raise NotImplementedError

    def validate(self, source: str) -> None:
        """Raise VtokError if `source` is not syntactically valid."""
        raise NotImplementedError

    def normalize(self, source: str) -> str:
        """Comment/docstring-free canonical source; baseline for fair compression stats."""
        return source
