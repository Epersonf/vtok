use super::statusline::{install_with, line, Installed};
use crate::usage::rec::Rec;
use serde_json::Value;

fn rec(kind: &str, raw: u64, shown: u64) -> Rec { Rec { t: 1, kind: kind.into(), raw, shown, tagged: true, ..Default::default() } }

fn scratch(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("vtok-line-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn the_line_gives_a_floor_and_a_ceiling_and_is_empty_without_calls() {
    let (run, read, find) = (rec("run", 6000, 900), rec("read", 9000, 3000), rec("find", 900_000, 400));
    assert_eq!(line(&[], None), "");
    assert_eq!(line(&[&run], None), "vtok saved 5.1k tok · 1 calls");
    assert_eq!(line(&[&run, &read, &find], Some("login")), "vtok[login] saved 5.1k-11.1k tok · 3 calls", "find is a call but never counted");
    assert_eq!(line(&[&read], None), "vtok saved up to 6.0k tok · 1 calls");
}

#[test]
fn the_status_line_is_added_once_and_the_rest_of_the_settings_stay() {
    let dir = scratch("add");
    let path = dir.join("settings.json");
    std::fs::write(&path, r#"{"model":"opus","hooks":{"Stop":[]}}"#).unwrap();
    let p = path.to_str().unwrap();
    assert_eq!(install_with(p, "vtok statusline").unwrap(), Installed::Added("vtok statusline".into()));
    let v: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!((v["statusLine"]["type"].as_str(), v["statusLine"]["command"].as_str(), v["model"].as_str()), (Some("command"), Some("vtok statusline"), Some("opus")));
    assert!(v["hooks"]["Stop"].is_array());
    assert_eq!(install_with(p, "vtok statusline").unwrap(), Installed::Present("vtok statusline".into()));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn another_status_line_and_a_broken_file_are_left_alone() {
    let dir = scratch("keep");
    let (mine, broken) = (dir.join("a.json"), dir.join("b.json"));
    std::fs::write(&mine, r#"{"statusLine":{"type":"command","command":"my-line.sh"}}"#).unwrap();
    std::fs::write(&broken, "{not json").unwrap();
    let before = std::fs::read_to_string(&mine).unwrap();
    assert_eq!(install_with(mine.to_str().unwrap(), "vtok statusline").unwrap(), Installed::Present("my-line.sh".into()));
    assert_eq!(std::fs::read_to_string(&mine).unwrap(), before);
    assert!(install_with(broken.to_str().unwrap(), "vtok statusline").unwrap_err().contains("left untouched"));
    assert_eq!(std::fs::read_to_string(&broken).unwrap(), "{not json");
    let _ = std::fs::remove_dir_all(&dir);
}
