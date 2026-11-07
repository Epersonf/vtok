//! The nearest `.vtok.json` wins over the user file; a value outside 1..=100000 is ignored.
use super::lookup::local_config_path;
use super::settings::MAX_LINES;

#[test]
fn local_beats_user_and_env_beats_both() {
    let d = std::env::temp_dir().join(format!("vtok-cfg-{}", std::process::id()));
    let sub = d.join("a/b");
    std::fs::create_dir_all(&sub).unwrap();
    std::fs::write(d.join(".vtok.json"), r#"{"max_lines": 77}"#).unwrap();
    let f = sub.join("x.rs");
    std::fs::write(&f, "fn x() {}\n").unwrap();
    assert_eq!(local_config_path(&f).unwrap(), d.join(".vtok.json"));
    assert_eq!(MAX_LINES.in_file(&d.join(".vtok.json")), Some(77));
    std::fs::write(d.join(".vtok.json"), r#"{"max_lines": 0}"#).unwrap();
    assert_eq!(MAX_LINES.in_file(&d.join(".vtok.json")), None);
    std::fs::remove_dir_all(&d).ok();
}
