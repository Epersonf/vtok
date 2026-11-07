//! `vtok refs`: definitions and uses found by name, comments and strings ignored.
use super::uses::{defines, find};

#[test]
fn finds_defs_and_code_uses_only() {
    let d = std::env::temp_dir().join(format!("vtok-refs-{}", std::process::id()));
    std::fs::create_dir_all(d.join("pkg")).unwrap();
    std::fs::write(d.join("pkg/a.py"), "def helper(x):\n    return x + 1\n\n# helper in a comment\nNOTE = \"helper in a string\"\n").unwrap();
    std::fs::write(d.join("pkg/b.py"), "from a import helper\n\ndef run():\n    return helper(2)\n").unwrap();
    std::fs::write(d.join("c.rs"), "fn other() {}\nfn go() { other(); }\n").unwrap();
    let r = find("helper", &d, 500).text;
    assert!(r.contains("1 def") && r.contains("def pkg/a.py helper L1-2"), "{r}");
    assert!(r.contains("use pkg/b.py run L3-4 @ L4"), "{r}");
    assert!(!r.contains("L4 ") || !r.contains("a.py run"), "{r}");
    assert!(!r.contains("@ L5") && !r.contains("L4,") , "comment and string mentions are not uses: {r}");
    let o = find("other", &d, 500).text;
    assert!(o.contains("def c.rs other L1") && o.contains("use c.rs go L2 @ L2"), "{o}");
    assert!(find("missing_name", &d, 500).text.contains("no definition found"));
    std::fs::remove_dir_all(&d).ok();
}

#[test]
fn defines_recognises_declarations() {
    assert!(defines("pub(crate) fn run(x: u8)", "run") && defines("class Session(Base):", "Session") && defines("export const run = 1", "run"));
    assert!(!defines("    run(1)", "run") && !defines("let x = run;", "run"));
}
