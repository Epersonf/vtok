"""Language-independent alias planning: exact breakeven (spec 4.3), stable ranking, collision-free names."""
import functools, re, string
from .tokens import ntok

TABLE_ROW_OVERHEAD = 2  # "=" and newline

@functools.lru_cache(maxsize=1)
def _single_token_words():
    """Short lowercase words that cost exactly one token in every position we emit identifiers."""
    try:
        import tiktoken
        enc = tiktoken.get_encoding("o200k_base")
    except Exception:
        return []
    words = []
    for i in range(enc.n_vocab):
        try: w = enc.decode_single_token_bytes(i).decode()
        except Exception: continue
        if re.fullmatch(r"[a-z]{2,4}", w) and len(enc.encode(w)) == 1 and len(enc.encode(" " + w)) == 1 \
           and len(enc.encode("(" + w + ",")) == 3:
            words.append(w)
    return sorted(words, key=lambda w: (len(w), w))

def alias_stream(avoid):
    """Cheapest names first: single letters, then one-token words, then letter+digit forms."""
    letters = string.ascii_lowercase
    def gen():
        yield from letters
        yield from string.ascii_uppercase
        yield from _single_token_words()
        for l in letters:
            for d in string.digits:
                yield l + d
        for a in letters:
            for b in letters + string.digits:
                yield a + b
        k = 0
        while True:
            for a in letters:
                yield f"{a}{k}{a}"
            k += 1
    return (a for a in gen() if a not in avoid)

def plan_aliases(counts, *, avoid, reserved=(), excluded=(), min_count=2):
    """counts: {name: occurrences}. Returns {original: alias}.
    reserved names are always aliased (they collide with Vtok tokens); excluded names are never
    aliased (imports/exports/etc.) and raise if also reserved."""
    avoid = set(avoid) | set(reserved)
    stream = alias_stream(avoid)
    forced, cands = [], []
    a_cost = 1   # aliases are single tokens (letters / one-token words)
    for name, n in counts.items():
        if name in reserved:
            forced.append(name); continue
        if name in excluded or n < min_count:
            continue
        L = ntok(name)
        saving = n * (L - a_cost) - (L + a_cost + TABLE_ROW_OVERHEAD)
        if saving > 0:
            cands.append((saving, name))
    table = {}
    for name in sorted(forced):
        table[name] = next(stream)
    for _, name in sorted(cands, key=lambda t: (-t[0], t[1])):
        table[name] = next(stream)
    return table
