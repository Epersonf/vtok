"""Vtok: compact code notation that cuts LLM context tokens (native Rust core)."""
import sys as _sys
from dataclasses import dataclass, field

from . import _vtok

__all__ = ["encode", "decode", "split", "chunk", "apply", "project_table", "languages", "grammar", "ntok",
           "Encoded", "Split", "VtokError", "main"]

VtokError = ValueError  # every error from the native layer is a ValueError


@dataclass
class Encoded:
    vtok: str
    table: dict = field(default_factory=dict)      # {original: alias}
    literals: dict = field(default_factory=dict)   # {placeholder: hidden literal} (never shown to the model)


@dataclass
class Split:
    index: str            # skeleton + chunk map, or the full view when the file is not split
    whole_file: bool
    table: dict
    chunks: list          # [(id, label, start_line, end_line)]


def languages():
    """[(name, extensions)] of the built-in language plugins."""
    return _vtok.languages()


def grammar(lang):
    """The short grammar prompt to give the LLM."""
    return _vtok.grammar(lang)


def ntok(text):
    """Token count (o200k_base, a proxy for Claude's tokenizer)."""
    return _vtok.ntok(text)


def encode(lang, source, *, alias=True, view="full", hide_literals=0, table=None, verify=True):
    """Source -> Vtok. view: 'full' (lossless) | 'skeleton' | 'lean' (read-only views)."""
    v, t, l = _vtok.encode(lang, source, alias, view, hide_literals, table, verify)
    return Encoded(v, t, l)


def decode(lang, vtok, table=None, literals=None):
    """Vtok -> source. `table`/`literals` are the dicts `encode` returned."""
    return _vtok.decode(lang, vtok, table, literals)


def split(lang, source, max_lines=150):
    i, w, t, c = _vtok.split(lang, source, max_lines)
    return Split(i, w, t, c)


def chunk(lang, source, id, max_lines=150):
    return _vtok.chunk(lang, source, id, max_lines)


def apply(lang, source, edits, max_lines=150, force=False):
    """Splice edited chunks ({id: vtok}) back into the original source."""
    return _vtok.apply(lang, source, edits, max_lines, force)


def project_table(lang, sources):
    """One stable alias table for many files of one language."""
    return _vtok.project_table(lang, list(sources))


def main(argv=None):
    """`vtok` command line."""
    args = list(_sys.argv[1:] if argv is None else argv)
    code = _vtok.run_cli(args)
    if argv is None:
        raise SystemExit(code)
    return code
