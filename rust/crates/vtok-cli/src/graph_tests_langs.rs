//! Python and TypeScript fixtures: call position, local shadowing, generic method names, imports, inheritance.
use super::tests::{put, tmp};
use super::tests_rels::{node, rel};
use super::*;

#[test]
fn python_calls_shadowing_and_inheritance() {
    let d = tmp("py");
    put(&d, "app/util.py", "def parse(s):\n    return s\n\ndef text():\n    return ''\n");
    put(&d, "app/models.py", "class Base:\n    def save(self):\n        return 1\n\nclass Repo(Base):\n    def load(self):\n        return self.save()\n");
    put(&d, "app/main.py", "from util import parse\nfrom models import Repo\n\ndef go(raw):\n    text = raw.strip()\n    repo = Repo()\n    n = text.count('a')\n    items = raw.split(',')\n    return parse(text) + repo.load() + len(items) + n\n");
    let g = build(&d);
    let go = node(&g, "go");
    assert_eq!(rel(&g, go, node(&g, "parse"), CALLS), Some(100), "imported and called");
    assert!(rel(&g, go, node(&g, "text"), CALLS).is_none() && rel(&g, go, node(&g, "text"), REFERENCES).is_none(), "`text` is a local here");
    assert_eq!(rel(&g, go, node(&g, "Repo"), USES_TYPE), Some(100), "Repo() constructs the imported class");
    assert_eq!(rel(&g, go, node(&g, "Repo.load"), CALLS), Some(100), "repo = Repo() types the receiver (one load method)");
    assert_eq!(rel(&g, node(&g, "Repo"), node(&g, "Base"), EXTENDS), Some(100));
    assert_eq!(rel(&g, node(&g, "Repo.load"), node(&g, "Base.save"), CALLS), None, "inherited self call stays unresolved rather than guessed");
    std::fs::remove_dir_all(&d).ok();
}

#[test]
fn typescript_imports_members_and_implements() {
    let d = tmp("ts");
    put(&d, "src/shape.ts", "export interface Shape { area(): number; }\nexport class Square implements Shape {\n  constructor(private s: number) {}\n  area(): number { return this.s * this.s; }\n  get(): number { return this.area(); }\n}\n");
    put(&d, "src/fmt.ts", "export function format(n: number): string { return String(n); }\nexport function push(x: number) { return x; }\n");
    put(&d, "src/main.ts", "import { Square } from './shape';\nimport { format } from './fmt';\nexport function show(xs: number[]): string {\n  const push = (n: number) => n;\n  const sq = new Square(2);\n  xs.push(1);\n  return format(sq.area() + push(3));\n}\n");
    let g = build(&d);
    let show = node(&g, "show");
    assert_eq!(rel(&g, show, node(&g, "format"), CALLS), Some(100));
    assert_eq!(rel(&g, show, node(&g, "Square"), USES_TYPE), Some(100), "new Square(..)");
    assert_eq!(rel(&g, show, node(&g, "Square.area"), CALLS), Some(100), "sq = new Square() types the receiver (one area method)");
    assert!(rel(&g, show, node(&g, "push"), CALLS).is_none(), "the local arrow function and xs.push are not fmt.push");
    assert_eq!(rel(&g, node(&g, "Square.get"), node(&g, "Square.area"), CALLS), Some(100), "this.area()");
    assert_eq!(rel(&g, node(&g, "Square"), node(&g, "Shape"), IMPLEMENTS), Some(100));
    std::fs::remove_dir_all(&d).ok();
}
