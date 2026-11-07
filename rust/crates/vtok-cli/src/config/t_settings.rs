//! The numeric settings: environment beats the nearest `.vtok.json`; a value outside its range is ignored.
use super::settings::{find, ALL, HOOK_WIDE, MAX_LINES, MIN_GAIN, RUN_TINY};
use super::{flag_or_number, shown};

fn project(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("vtok-set-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn a_value_outside_the_range_is_ignored() {
    assert_eq!(MAX_LINES.parse("0"), None);
    assert_eq!(MAX_LINES.parse("100001"), None);
    assert_eq!(MIN_GAIN.parse("0"), Some(0));
    assert_eq!(MIN_GAIN.parse(" 25 "), Some(25));
    assert_eq!(MIN_GAIN.parse("91"), None);
    assert_eq!(MIN_GAIN.parse("-3"), None);
    assert_eq!(RUN_TINY.parse("20001"), None);
    assert_eq!(RUN_TINY.parse("many"), None);
}

#[test]
fn the_environment_beats_the_nearest_file_and_a_bad_one_falls_through() {
    let d = project("env");
    std::fs::write(d.join(".vtok.json"), r#"{"min_gain": 25, "run_tiny": 5}"#).unwrap();
    let f = d.join("x.rs");
    std::fs::write(&f, "fn x() {}\n").unwrap();
    assert_eq!(MIN_GAIN.get_with(Some("40"), Some(&f)), (40, "VTOK_MIN_GAIN".to_string()));
    assert_eq!(MIN_GAIN.get_with(None, Some(&f)).0, 25);
    assert_eq!(MIN_GAIN.get_with(Some("999"), Some(&f)).0, 25);
    assert_eq!(RUN_TINY.get_with(None, Some(&f)).0, 5);
    // a key the file does not have comes from further out; here at most the user file or the default
    assert_ne!(MAX_LINES.get_with(None, Some(&f)).1, d.join(".vtok.json").display().to_string());
    std::fs::write(d.join(".vtok.json"), r#"{"min_gain": 95}"#).unwrap();
    assert_ne!(MIN_GAIN.get_with(None, Some(&f)).1, d.join(".vtok.json").display().to_string());
    std::fs::remove_dir_all(&d).ok();
}

#[test]
fn the_wide_hook_setting_is_off_by_default_and_reads_on_and_off() {
    assert_eq!((HOOK_WIDE.default, HOOK_WIDE.parse("1"), HOOK_WIDE.parse("2")), (0, Some(1), None));
    assert_eq!(find("hook-wide").map(|s| s.env), Some("VTOK_HOOK_WIDE"));
    assert_eq!([flag_or_number(&HOOK_WIDE, "on"), flag_or_number(&HOOK_WIDE, "off"), flag_or_number(&HOOK_WIDE, "1")], [Some(1), Some(0), Some(1)]);
    assert_eq!(flag_or_number(&MIN_GAIN, "on"), None, "only a two-valued setting reads words");
    assert_eq!((shown(&HOOK_WIDE, 1), shown(&HOOK_WIDE, 0), shown(&MIN_GAIN, 10)), ("on".to_string(), "off".to_string(), "10".to_string()));
    assert_eq!(HOOK_WIDE.get_with(Some("1"), None), (1, "VTOK_HOOK_WIDE".to_string()));
}

#[test]
fn settings_are_found_by_either_spelling() {
    assert_eq!(find("min-gain").map(|s| s.key), Some("min_gain"));
    assert_eq!(find("min_gain").map(|s| s.name), Some("min-gain"));
    assert!(find("mcp-run").is_none() && find("nope").is_none());
    assert_eq!((MAX_LINES.default, MIN_GAIN.default, RUN_TINY.default), (150, 10, 250));
    assert!(ALL.iter().all(|s| s.min <= s.default && s.default <= s.max));
}
