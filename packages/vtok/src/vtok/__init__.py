from .errors import VtokError
from .language import Language, Encoded, Analysis, Unit
from .split import plan_chunks, file_table, render_index, chunk_vtok, apply_edits, chunk_source
from .project import plan_project
from .registry import available, get_language, language_for_path
from .table import render_table, parse_table
from .tokens import ntok

def encode(source, lang, **kw):
    return get_language(lang).encode(source, **kw)

def decode(vtok, lang, table=None, literals=None):
    return get_language(lang).decode(vtok, table, literals)
