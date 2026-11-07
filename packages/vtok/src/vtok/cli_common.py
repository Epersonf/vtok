"""What every command needs: the language of a file and the files under a path."""
import pathlib
from . import registry

def _lang(a, path=None):
    if getattr(a, "lang", None):
        return registry.get_language(a.lang)
    return registry.language_for_path(path)

def _files(paths, lang_name=None):
    exts = None
    if lang_name:
        exts = registry.get_language(lang_name).extensions
    for p in paths:
        p = pathlib.Path(p)
        if p.is_dir():
            for f in sorted(x for x in p.rglob("*") if x.is_file()):
                if (exts and f.suffix.lower() in exts) or (not exts and f.suffix.lower() in
                        {e for l in registry.available().values() for e in l.extensions}):
                    yield f
        else:
            yield p
