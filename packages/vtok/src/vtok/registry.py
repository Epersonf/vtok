"""Language plugin discovery via entry points (group `vtok.languages`)."""
import pathlib
from importlib.metadata import entry_points

GROUP = "vtok.languages"
KNOWN = {  # languages we ship plugins for -> pip package
    "python": "vtok-lang-python",
    "typescript": "vtok-lang-typescript",
}
_cache = {}

def available():
    """{name: Language instance} for every installed plugin."""
    if not _cache:
        for ep in entry_points(group=GROUP):
            try:
                obj = ep.load()
                _cache[ep.name] = obj() if isinstance(obj, type) else obj
            except Exception as e:  # broken plugin must not break the others
                _cache[ep.name] = e
    return {k: v for k, v in _cache.items() if not isinstance(v, Exception)}

def broken():
    available()
    return {k: v for k, v in _cache.items() if isinstance(v, Exception)}

def get_language(name):
    langs = available()
    if name in langs:
        return langs[name]
    hint = f" (install it: pip install {KNOWN[name]})" if name in KNOWN else ""
    raise LookupError(f"language {name!r} not installed{hint}; installed: {sorted(langs) or 'none'}")

def language_for_path(path):
    ext = pathlib.Path(path).suffix.lower()
    for lang in available().values():
        if ext in lang.extensions:
            return lang
    known = [n for n, p in KNOWN.items()]
    raise LookupError(f"no installed plugin handles {ext!r}; installed: {sorted(available()) or 'none'}; known plugins: {known}")
