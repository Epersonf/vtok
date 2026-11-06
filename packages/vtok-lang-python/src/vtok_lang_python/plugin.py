import ast
from vtok.language import Language, Encoded, Analysis
from .encode import encode as _encode, strip_docstrings
from .aliases import analyze_tree, prepare_forced_imports
from .grammar import RESERVED
from . import chunks as _ch
from .decode import decode as _decode
from .grammar import GRAMMAR_PROMPT

class Python(Language):
    name = "python"
    extensions = (".py", ".pyi")
    grammar_prompt = GRAMMAR_PROMPT

    def analyze(self, source):
        tree = strip_docstrings(ast.parse(source))
        prepare_forced_imports(tree)
        counts, excluded, avoid = analyze_tree(tree)
        return Analysis(counts, excluded, avoid, set(RESERVED))

    def encode(self, source, *, alias=True, verify=True, table=None, view="full", hide_literals=0):
        lits = {}
        vt, table = _encode(source, alias=alias, verify=verify, table=table, view=view, hide=hide_literals, lits_out=lits)
        return Encoded(vt, table, lits)

    def decode(self, vtok, table=None, literals=None):
        return _decode(vtok, table, literals=literals)

    def normalize(self, source):
        return ast.unparse(strip_docstrings(ast.parse(source)))

    def units(self, source, max_lines):
        return _ch.units(source, max_lines)

    def encode_chunk(self, text, table, member):
        return _ch.encode_chunk(self, text, table, member)

    def decode_chunk(self, vtok, table, member, indent, original=None):
        return _ch.decode_chunk(vtok, table, member, indent, original)

    def equivalent(self, a, b, member):
        return _ch.equivalent(a, b, member)

    def validate(self, source):
        _ch.validate(source)
