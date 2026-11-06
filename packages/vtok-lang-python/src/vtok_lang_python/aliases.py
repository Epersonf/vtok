"""Alias table: deterministic, collision-free, exact breakeven (spec 4.3)."""
import ast, builtins, keyword
from collections import Counter
from .grammar import RESERVED
from vtok.aliasing import plan_aliases

def iter_renamable(tree):
    """Yield (node, attr) pairs whose string value is a renamable identifier.
    Only Name.id, arg.arg, module/function-level def names, non-member classes,
    except-as names and global/nonlocal names. Never attributes or keyword args
    (they may refer to external APIs); class-member defs excluded (accessed via attr)."""
    def walk(node, in_class):
        for child in ast.iter_child_nodes(node):
            yield from visit(child, node, in_class)
    def visit(n, parent, in_class):
        if isinstance(n, ast.Name):
            yield n, "id"
        elif isinstance(n, ast.arg):
            yield n, "arg"
        elif isinstance(n, ast.Attribute):
            yield n, "attr"
        elif isinstance(n, ast.keyword) and n.arg:
            yield n, "arg"
        elif isinstance(n, (ast.FunctionDef, ast.AsyncFunctionDef)):
            yield n, "name"
            yield from walk(n, False)
            return
        elif isinstance(n, ast.ClassDef):
            yield n, "name"
            yield from walk(n, True)
            return
        elif isinstance(n, ast.ExceptHandler) and n.name:
            yield n, "name"
        elif isinstance(n, ast.alias) and n.asname:
            yield n, "asname"
        elif type(n).__name__ in ("TypeVar", "ParamSpec", "TypeVarTuple"):
            yield n, "name"
        elif isinstance(n, (ast.Global, ast.Nonlocal)):
            yield n, "names"
        yield from walk(n, in_class)
    yield from visit(tree, None, False)

def _names(node, attr):
    v = getattr(node, attr)
    return v if isinstance(v, list) else [v]

def all_identifiers(tree):
    s = set()
    for n in ast.walk(tree):
        for a in ("id", "arg", "name", "attr"):
            v = getattr(n, a, None)
            if isinstance(v, str): s.add(v)
        if isinstance(n, ast.keyword) and n.arg: s.add(n.arg)
        if isinstance(n, (ast.Global, ast.Nonlocal)): s.update(n.names)
        if isinstance(n, ast.alias):
            s.add(n.name); s.update(n.name.split(".")); 
            if n.asname: s.add(n.asname)
    return s

def imported_names(tree):
    s = set()
    for n in ast.walk(tree):
        if isinstance(n, ast.alias):
            s.add(n.asname or n.name.split(".")[0])
    return s

def analyze_tree(tree, extra_avoid=()):
    counts = Counter()
    for node, attr in iter_renamable(tree):
        for nm in _names(node, attr):
            counts[nm] += 1
    avoid = all_identifiers(tree) | set(keyword.kwlist) | set(dir(builtins)) | set(extra_avoid)
    return counts, imported_names(tree) - RESERVED, avoid

def build_table(tree, extra_avoid=(), min_count=2):
    counts, excluded, avoid = analyze_tree(tree, extra_avoid)
    return plan_aliases(counts, avoid=avoid, reserved=RESERVED, excluded=excluded, min_count=min_count)

def rename(tree, mapping):
    for node, attr in iter_renamable(tree):
        v = getattr(node, attr)
        if isinstance(v, list):
            setattr(node, attr, [mapping.get(x, x) for x in v])
        else:
            setattr(node, attr, mapping.get(v, v))

def prepare_forced_imports(tree):
    """`from x import f` where f collides with a reserved token -> add explicit asname so it can be aliased."""
    for n in ast.walk(tree):
        if isinstance(n, ast.alias) and not n.asname and "." not in n.name and n.name in RESERVED:
            n.asname = n.name

def restore_imports(tree, forced_nodes):
    for n in forced_nodes:
        if n.asname == n.name:
            n.asname = None
