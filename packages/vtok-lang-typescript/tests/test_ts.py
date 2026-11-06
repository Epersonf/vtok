import pytest
from vtok import VtokError
from vtok_lang_typescript import TypeScript, encode, decode

@pytest.mark.parametrize("src", [
    "export function fn(a: string, b?: number): boolean { if (!a || b! === 1) { return false } else if (b) return true; else { b++ } return a === 'x' }",
    "switch (x) { case 1: f(); break; default: g() }",
    "try { a() } catch (e) { b() } finally { c() }",
    "try { a() } catch { }",
    "export default class A<T> extends B implements C { @dec() x: number = 1; static async foo(a: T): Promise<void> {} get y() { return 1 } }",
    "enum E { A = 1, B }\ninterface I { a: number; b(): void }\ntype K = `${A}_${B}`;",
    "const t = `a${b}\nc`; const o = { a, b: 1 }; const { a: aa, c } = o;",
    "for (let i = 0; i < n; i++) { if (i) continue }\nfor (const x of y) z(x)\nwhile (a) { b() }\ndo { x() } while (y)",
    "import { a } from 'm'; export { a }; export * from 'n';",
    "const f = (a: number) => { return a + 1 }; label: for (;;) { break label }",
    "function g() {}\nclass H {}\nif (x) {}",
])
def test_roundtrip(src):
    encode(src, verify=True)
    encode(src, verify=True, keywords=False)

def test_reserved_names_get_aliased():
    encode("function g(f: number, r: number, k = 1, e = 2) { let l = f + r + k + e; return l }")

def test_decode_error_has_vtok_line():
    with pytest.raises(VtokError) as e:
        decode("f foo(\n r 1\n")
    assert e.value.line is not None

def test_plugin_registered():
    from vtok import registry
    assert "typescript" in registry.available()

def test_skeleton_elides_bodies():
    vt, _ = encode("class A { m(a: number) { return a + 1 }\n n = () => { return 2 } }", view="skeleton")
    assert "a+1" not in vt and "return 2" not in vt
    decode(vt)

BIG = "import { a } from 'x';\nexport class Big {\n" + "".join(
    f"  // c{i}\n  m{i}(x: number): number {{\n    return x + {i};\n  }}\n" for i in range(12)) + "}\n"

def test_ts_split_edit_roundtrip():
    import vtok
    L = vtok.registry.get_language("typescript")
    t = vtok.file_table(L, BIG)
    ch = vtok.plan_chunks(L, BIG, 12)
    assert len(ch) > 1 and any(c.member for c in ch)
    out = vtok.apply_edits(L, BIG, ch, {c.id: vtok.chunk_vtok(L, BIG, c, t) for c in ch}, t)
    assert out == BIG                              # nothing changed -> byte-identical
    tgt = next(c for c in ch if "m5" in c.label)
    vt = vtok.chunk_vtok(L, BIG, tgt, t).replace("+5", "*5")
    out = vtok.apply_edits(L, BIG, ch, {tgt.id: vt}, t)
    assert "x*5" in out.replace(" ", "") and "// c0" in out

@pytest.mark.parametrize("src", [
    "for (const x of xs) { f(x) }\nfor (let i = 0; i < n; i++) { g(i) }\nfor (const [a, b] of m) h(a, b)",
    "for (;;) { break }\nfor (const k in o) {}\nasync function q() { for await (const x of s) { y(x) } }",
    "for (x of xs) { z(x) }\nvar i; for (i = 0; i < 3; i++) { w() }",
])
def test_for_header_without_parens(src):
    vt, _ = encode(src)
    assert "L (" not in vt
    encode(src, keywords=False)

def test_for_decode_accepts_explicit_parens_and_loose_form():
    for vt in ("L k x of xs\n f(x)\n", "L (const x of xs)\n f(x)\n"):
        assert "for (const x of xs)" in decode(vt, style="legacy")


def test_hidden_literals_roundtrip_skip_types_and_imports():
    from vtok_lang_typescript import TypeScript
    L = TypeScript()
    src = 'import {a} from "./some/long/module/path";\ntype K = "alpha"|"beta";\nconst m = "a rather long error message that costs tokens";\n'
    e = L.encode(src, hide_literals=5)
    assert "rather long" not in e.vtok and "some/long" in e.vtok and '"alpha"' in e.vtok
    L.decode(e.vtok, {a: o for o, a in e.table.items()}, e.literals)


def test_lean_view_drops_types_and_import_paths():
    from vtok_lang_typescript import TypeScript
    e = TypeScript().encode('import {a} from "./p";\nfunction f(x: number): string { return a(x) }\n', view="lean")
    assert "number" not in e.vtok and "./p" not in e.vtok
