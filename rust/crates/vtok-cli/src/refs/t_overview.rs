//! `vtok map`: every file listed, symbols ranked by the uses outside their own file.
use super::overview::map;

#[test]
fn map_lists_every_file_and_ranks_used_symbols() {
    let d = std::env::temp_dir().join(format!("vtok-map-{}", std::process::id()));
    std::fs::create_dir_all(d.join("pkg")).unwrap();
    std::fs::write(d.join("pkg/core.py"), "def shared(x):\n    return x\n\ndef lonely():\n    return 1\n").unwrap();
    std::fs::write(d.join("pkg/a.py"), "from core import shared\n\ndef a():\n    return shared(1)\n").unwrap();
    std::fs::write(d.join("pkg/b.py"), "from core import shared\n\ndef b():\n    return shared(2) + shared(3)\n").unwrap();
    let m = map(&d, 400).text;
    assert!(m.contains("pkg/") && m.contains("core.py") && m.contains("a.py") && m.contains("b.py"), "{m}");
    let core = m.lines().find(|l| l.contains("core.py")).unwrap();
    assert!(core.contains("shared") && core.contains("x3"), "shared is used 3 times elsewhere: {core}");
    std::fs::remove_dir_all(&d).ok();
}
