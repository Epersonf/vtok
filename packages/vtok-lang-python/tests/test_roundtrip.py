import ast, pathlib, pytest
from vtok_lang_python import encode, decode
from vtok import VtokError

REF = """C AuthMgr
 f val(u,p)->b
  ?!u|!p r F
  r db.q(u,p)
"""

def same(a, b): return ast.dump(ast.parse(a)) == ast.dump(ast.parse(b))

def test_reference_decode():
    out = decode(REF, style="legacy")
    assert same(out, "class AuthMgr:\n def val(u,p)->bool:\n  if not u or not p: return False\n  return db.q(u,p)\n")

def test_roundtrip_example():
    src = pathlib.Path(__file__).parent.parent.parent.parent.joinpath("examples/auth.py").read_text()
    vt, t = encode(src)
    assert "if not user or not password:return False" in vt

@pytest.mark.parametrize("src", [
    "x = a | b\ny = a & b\nz = a or b and not c\n",
    "def f(r, T, F, e):\n    r = r + T\n    return F if e else None\n",
    "from x import f\nf(1)\n",
    "import builtins as m\nm = 1\n",
    "async def g(x):\n    async for i in x:\n        await i\n",
    "x = f'{a[\"k\"]} {', '.join(b)}'\n",
    "try:\n    pass\nexcept (A, B) as e:\n    raise\nfinally:\n    pass\n",
    "for i in range(3):\n    if i % 2:\n        continue\n    else:\n        break\n",
    "x = 3 .real\n",
    "assert a is not None, 'msg'\nif a not in b: pass\n",
])
def test_roundtrip_snippets(src):
    encode(src, verify=True)  # raises on AST mismatch

def test_decode_error_has_line_number():
    with pytest.raises(VtokError) as e:
        decode("C A\n f x(\n  r 1\n", style="legacy")
    assert e.value.line is not None

def test_new_names_pass_through():
    out = decode("f brand_new(x)\n r x\n", {"a0": "other"}, style="legacy")
    assert "def brand_new" in out

def test_skeleton_elides_bodies_and_stays_valid():
    src = "class A:\n    def f(self, x):\n        return x + 1\n    def g(self):\n        pass\n"
    vt, _ = encode(src, view="skeleton")
    assert "x+1" not in vt and "..." in vt
    ast.parse(decode(vt))

def test_project_table_is_shared_and_verified():
    from vtok import plan_project, registry
    L = registry.get_language("python")
    srcs = ["def long_function_name(a):\n    return long_function_name(a)+long_function_name(a)\n"] * 3
    tbl = plan_project(L, srcs)
    for s in srcs:
        L.encode(s, table=tbl)

BIG = "import os\n\nclass Big:\n    '''Doc.'''\n" + "".join(
    f"    def m{i}(self, x):\n        '''doc {i}'''\n        # comment {i}\n        return x + {i}\n\n" for i in range(12))

def _setup(max_lines=20):
    import vtok
    L = vtok.registry.get_language("python")
    t = vtok.file_table(L, BIG)
    return vtok, L, t, vtok.plan_chunks(L, BIG, max_lines)

def test_split_plans_member_chunks():
    vtok, L, t, ch = _setup()
    assert len(ch) > 1 and all(c.lines <= 20 for c in ch)
    assert any(c.member for c in ch)

def test_edit_one_chunk_preserves_the_rest_byte_for_byte():
    vtok, L, t, ch = _setup()
    target = next(c for c in ch if "m5" in c.label)
    vt = vtok.chunk_vtok(L, BIG, target, t)
    edited = vt.replace("+5", "*5")
    assert edited != vt
    out = vtok.apply_edits(L, BIG, ch, {target.id: edited}, t)
    assert "x * 5" in out and "# comment 0" in out and "doc 5" in out
    first = BIG.splitlines()[: target.start - 1]
    assert out.splitlines()[: target.start - 1] == first      # untouched prefix identical

def test_unchanged_chunk_is_not_rewritten():
    vtok, L, t, ch = _setup()
    out = vtok.apply_edits(L, BIG, ch, {c.id: vtok.chunk_vtok(L, BIG, c, t) for c in ch}, t)
    assert out == BIG

def test_force_roundtrip_is_ast_equal():
    vtok, L, t, ch = _setup()
    out = vtok.apply_edits(L, BIG, ch, {c.id: vtok.chunk_vtok(L, BIG, c, t) for c in ch}, t, force=True)
    assert ast.dump(ast.parse(out)) == ast.dump(ast.parse(BIG))

def test_model_variable_names_that_look_like_keywords():
    out = decode("f g(xs)\n L p in xs\n  p.append(1)\n  p=p+1\n r p\n", style="legacy")
    assert same(out, "def g(xs):\n for p in xs:\n  p.append(1)\n  p=p+1\n return p\n")
    assert same(decode("f g()\n r=1\n t,u=1,2\n E=3\n r r\n", style="legacy"), "def g():\n r=1\n t,u=1,2\n E=3\n return r\n")

def test_inline_body_accepts_any_simple_statement():
    out = decode("f g(a,out)\n ?a\n  out.append(1)\n E out[0]=2\n ?a>1 out=[]\n r out\n", style="legacy")
    assert same(out, "def g(a,out):\n if a:\n  out.append(1)\n else: out[0]=2\n if a>1: out=[]\n return out\n")

def test_else_question_is_elif():
    out = decode("f g(a,b)\n ?a\n  r 1\n E ?b\n  r 2\n E\n  r 3\n", style="legacy")
    assert same(out, "def g(a,b):\n if a:\n  return 1\n elif b:\n  return 2\n else:\n  return 3\n")

def test_expression_errors_carry_line_numbers():
    with pytest.raises(VtokError) as e:
        decode("f g()\n r 1\n x=1 ? 2\n", style="legacy")
    assert e.value.line == 3


def test_hidden_literals_roundtrip_and_shrink():
    from vtok_lang_python import Python
    L = Python()
    src = "def f(x):\n    msg = 'a rather long error message that costs several tokens'\n    return msg + 'short'\n"
    e = L.encode(src, hide_literals=5)
    assert "rather long" not in e.vtok and e.literals
    back = L.decode(e.vtok, {a: o for o, a in e.table.items()}, e.literals)
    assert ast.dump(ast.parse(back)) == ast.dump(ast.parse(src))


def test_lean_view_drops_annotations():
    from vtok_lang_python import Python
    e = Python().encode("def f(x: int) -> str:\n    y: int = 1\n    return str(x)\n", view="lean")
    assert "int" not in e.vtok.replace("str(x)", "")
