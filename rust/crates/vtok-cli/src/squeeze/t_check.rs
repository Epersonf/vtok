//! `--check`: protected items that a rewrite lost are reported.
use super::check::missing;

#[test]
fn check_reports_lost_protected_items() {
    let old = "# Build\nRun `make all` then see https://a.b/c.\n```sh\ncargo test\n```\n";
    assert!(missing(old, "# Build\nRun `make all`, see https://a.b/c.\n```sh\ncargo test\n```\n").is_empty());
    let m = missing(old, "# Build\nRun make.\n");
    assert!(m.iter().any(|x| x.contains("make all")) && m.iter().any(|x| x.contains("https://a.b/c")) && m.iter().any(|x| x.contains("cargo test")), "{m:?}");
}
