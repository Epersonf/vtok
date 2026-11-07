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


if __name__ == "__main__":
    main(sys.argv[1] if len(sys.argv) > 1 else "fixtures")
