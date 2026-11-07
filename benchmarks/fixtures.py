"""Create the failing sample projects the benchmark runs commands in.

usage: python fixtures.py DIR
Needs cargo, python (pytest, mypy, ruff), node (npm: typescript, eslint@8, vitest) and go on PATH.
"""
import os
import subprocess
import sys


def w(path, text):
    os.makedirs(os.path.dirname(path) or ".", exist_ok=True)
    open(path, "w", encoding="utf-8").write(text)


def sh(cmd, cwd):
    subprocess.run(cmd, cwd=cwd, check=True, shell=(os.name == "nt"), capture_output=True)


def main(d):
    os.makedirs(d, exist_ok=True)
    # Rust: compile errors, and 40 tests of which 3 fail
    sh(["cargo", "new", "-q", "--lib", "rsfail"], d)
    w(f"{d}/rsfail/src/lib.rs", "pub fn bad() -> u32 { let x: String = 5; let unused = 1; x.len() }\n"
      "pub fn bad2(v: Vec<i32>) -> i32 { v.iter().sum::<i32>() + missing_fn() }\n")
    sh(["cargo", "new", "-q", "--lib", "rstest"], d)
    t = "pub fn f(a:i32)->i32{a*2}\n#[cfg(test)]\nmod t{use super::*;\n"
    for i in range(40):
        t += f"#[test] fn t{i}(){{ assert_eq!(f({i}), {1 if i in (7, 19, 33) else i * 2}); }}\n"
    w(f"{d}/rstest/src/lib.rs", t + "}\n")
    # Python: 40 tests (3 fail), type errors, lint errors
    t = "".join(f"def test_{i}():\n    assert {i} * 2 == {1 if i in (5, 17, 29) else i * 2}\n" for i in range(40))
    w(f"{d}/py/tests/test_a.py", t)
    w(f"{d}/py/m.py", "def f(x: int) -> str:\n    return x\n\ny: int = 'a'\nimport os\nimport sys\n")
    # TypeScript: type errors, 30 vitest cases (2 fail), eslint violations
    w(f"{d}/ts/tsconfig.json", '{"compilerOptions":{"strict":true,"noEmit":true,"target":"es2020","module":"esnext",'
      '"moduleResolution":"bundler"},"include":["*.ts"]}')
    w(f"{d}/ts/a.ts", "".join(f'export const v{i}: number = {chr(34) + "x" + chr(34) if i % 3 == 0 else i};\n'
                              f"export function f{i}(a: string): number {{ return a; }}\n" for i in range(25)))
    w(f"{d}/ts/a.test.ts", 'import {test,expect} from "vitest";\n' + "".join(
        f'test("case {i}", ()=>{{ expect({i}+1).toBe({0 if i in (4, 11) else i + 1}); }});\n' for i in range(30)))
    w(f"{d}/ts/b.js", "".join(f"var a{i} = {i}\nfunction f{i}(){{ return b{i} }}\nif (a{i} == 1) {{}}\n" for i in range(30)))
    w(f"{d}/ts/.eslintrc.json", '{"root":true,"env":{"es2020":true,"node":true},"extends":"eslint:recommended",'
      '"parserOptions":{"ecmaVersion":2020,"sourceType":"module"},"rules":{"eqeqeq":"error","no-var":"error"}}')
    w(f"{d}/ts/package.json", '{"name":"fx","private":true}')
    sh(["npm", "i", "-s", "typescript", "eslint@8", "vitest"], f"{d}/ts")
    # Go: 30 tests, 2 fail
    w(f"{d}/gof/go.mod", "module ex.com/gof\n\ngo 1.21\n")
    w(f"{d}/gof/a.go", "package gof\nfunc F(a int) int { return a*2 }\n")
    w(f"{d}/gof/a_test.go", 'package gof\nimport "testing"\n' + "".join(
        f'func TestF{i}(t *testing.T){{ if F({i}) != {1 if i in (3, 9) else i * 2} {{ t.Errorf("F({i}) wrong") }} }}\n'
        for i in range(30)))
    # Mixed, realistic failures (varied messages, unlike the repeated ones above)
    ts_mixed = ["interface User { id: number; name: string; email?: string }",
                "class A { private p = 1; static make(n: number): A { return new A(); } }"]
    for i in range(12):
        ts_mixed += [f"export function f{i}(u: User): string {{ return u.nme{i}; }}",
                     f"export const n{i}: number = {chr(34)}text{i}{chr(34)};",
                     f"export const m{i} = missingName{i} + {i};"]
    ts_mixed += ["export function g(a: number, b: string): void {}", "g(1);", "g('x', 2);", "g(1, 'a', 3);",
                 "new A().p;", "A.make('1');", "const u: User = { id: 1 };", "import { nothing } from './nonexistent';",
                 "const arr: string[] = [1, 2, 3];", "let v: boolean = null as any as number;"]
    w(f"{d}/ts2/mixed.ts", chr(10).join(ts_mixed) + chr(10))
    w(f"{d}/ts2/tsconfig.json", '{"compilerOptions":{"strict":true,"noEmit":true,"target":"es2020","module":"esnext",'
      '"moduleResolution":"bundler"},"include":["*.ts"]}')
    py = ["import pytest", "", "def helper(d, k):", "    return d[k]", "", "def divide(a, b):", "    return a / b", ""]
    cases = ["assert helper({'a': 1}, 'b') == 1", "assert divide(1, 0) == 0", "assert [1, 2, 3] == [1, 2, 4]",
             "assert 'abc'.upper() == 'abd'", "assert {'x': 1} == {'x': 2}", "raise ValueError('bad config: timeout=-1')",
             "assert len('hello') == 6", "assert None is not None", "assert isinstance(3, str)",
             "assert abs(-3) == -3"]
    for i in range(30):
        body = cases[i % len(cases)] if i % 3 == 0 else f"assert {i} + {i} == {2 * i}"
        py += [f"def test_case_{i}():", f"    {body}", ""]
    py += ["@pytest.mark.parametrize('x,y', [(1, 2), (2, 4), (3, 7)])", "def test_double(x, y):", "    assert x * 2 == y", ""]
    w(f"{d}/py2/tests/test_mixed.py", chr(10).join(py))
    # A large source file at a pinned revision, for the `cat` and `grep` scenarios (needs this repository's git history).
    repo = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    src = subprocess.run(["git", "show", "57c4ad1:rust/crates/vtok-cli/src/shell.rs"], cwd=repo, capture_output=True, check=True).stdout
    open(f"{d}/big.rs", "wb").write(src)


if __name__ == "__main__":
    main(sys.argv[1] if len(sys.argv) > 1 else "fixtures")
