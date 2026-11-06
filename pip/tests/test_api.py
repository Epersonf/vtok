import pytest
import vtok

TS = """// auth
export class AuthMgr {
  constructor(private db: Db) {}
  validate(user: string, password: string): boolean {
    if (!user || !password) {
      return false;
    }
    return this.db.query(user, password);
  }
}
"""
PY = '''"""Pagination."""

def paginate(items, page, size):
    """One page."""
    if page < 1:
        raise ValueError("page must be >= 1")
    start = (page - 1) * size
    return list(items[start:start + size])
'''


def test_languages():
    assert sorted(n for n, _ in vtok.languages()) == ["python", "typescript"]


def test_roundtrip_typescript():
    r = vtok.encode("typescript", TS)
    assert len(r.vtok) < len(TS) and "// auth" not in r.vtok
    assert "validate(user:string,password:string):boolean" in vtok.decode("typescript", r.vtok, r.table, r.literals)


def test_python_compact_and_readable_decode():
    r = vtok.encode("python", PY, alias=False)
    assert "if page<1:raise ValueError" in r.vtok and "Pagination" not in r.vtok
    assert "def paginate(items,page,size):\n    if page<1:" in vtok.decode("python", r.vtok)


def test_hidden_literals():
    src = "def f():\n    return 'a rather long error message that costs several tokens'\n"
    r = vtok.encode("python", src, hide_literals=5)
    assert "rather long" not in r.vtok and len(r.literals) == 1
    assert "rather long error message" in vtok.decode("python", r.vtok, r.table, r.literals)


def test_views():
    assert "query" not in vtok.encode("typescript", TS, view="skeleton").vtok
    assert "string" not in vtok.encode("typescript", TS, view="lean").vtok


def test_split_chunk_apply():
    big = "".join(f"def f{i}(x):\n    y = x + {i}\n    return y * 2\n\n" for i in range(40))
    s = vtok.split("python", big, 30)
    assert not s.whole_file and len(s.chunks) > 1
    cid = s.chunks[0][0]
    c = vtok.chunk("python", big, cid, 30)
    assert vtok.apply("python", big, {cid: c}, 30) == big
    out = vtok.apply("python", big, {cid: c.replace("+0", "+100")}, 30)
    assert "y=x+100" in out


def test_errors_are_value_errors():
    with pytest.raises(ValueError, match="syntax"):
        vtok.encode("python", "def (:")
    with pytest.raises(ValueError, match="unknown language"):
        vtok.encode("cobol", "x")


def test_project_table():
    t = vtok.project_table("python", [PY, PY.replace("paginate", "paginate2")])
    assert isinstance(vtok.encode("python", PY, table=t).vtok, str)


def test_cli_entry():
    assert vtok.main(["languages"]) == 0
