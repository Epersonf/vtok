from vtok.language import Language, Encoded, Analysis
from .encode import encode as _encode, _excluded_and_counts
from .grammar import RESERVED
from .decode import decode as _decode
from .grammar import GRAMMAR_PROMPT
from .parse import parse
from . import chunks as _ch

class TypeScript(Language):
    name = "typescript"
    extensions = (".ts", ".mts", ".cts")
    grammar_prompt = GRAMMAR_PROMPT

    def __init__(self, keywords=True):
        self.keywords = keywords

    def analyze(self, source):
        counts, excl, avoid = _excluded_and_counts(parse(source.replace("\r\n", "\n").encode()).root_node)
        return Analysis(counts, excl, avoid, set(RESERVED))

    def encode(self, source, *, alias=True, verify=True, table=None, view="full", hide_literals=0):
        lits = {}
        vt, table = _encode(source, alias=alias, verify=verify, keywords=self.keywords, table=table, view=view, hide=hide_literals, lits_out=lits)
        return Encoded(vt, table, lits)

    def decode(self, vtok, table=None, literals=None):
        return _decode(vtok, table, literals=literals)

    def normalize(self, source):
        raw = source.replace("\r\n", "\n").encode()
        spans, stack = [], [parse(raw).root_node]
        while stack:
            n = stack.pop(); stack.extend(n.children)
            if n.type == "comment": spans.append((n.start_byte, n.end_byte))
        for a, b in sorted(spans, reverse=True): raw = raw[:a] + raw[b:]
        return raw.decode()

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
