use super::state::resolve;
use std::ffi::OsString;
use std::path::PathBuf;

fn env<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<OsString> + 'a {
    move |k| pairs.iter().find(|(n, _)| *n == k).map(|(_, v)| OsString::from(v))
}

fn p(root: &str, rest: &str) -> PathBuf { PathBuf::from(root).join(rest) }

#[test]
fn explicit_settings_win_and_stand_alone() {
    let all = [("VTOK_STATE", "/s"), ("XDG_STATE_HOME", "/x"), ("HOME", "/h"), ("LOCALAPPDATA", "/l")];
    assert_eq!(resolve(&env(&all), true), vec![PathBuf::from("/s")]);
    assert_eq!(resolve(&env(&all[1..]), true), vec![p("/x", "vtok")]);
}

#[test]
fn windows_writes_to_local_app_data_even_when_home_is_set_and_still_reads_home() {
    let e = [("HOME", "C:\\Users\\me"), ("USERPROFILE", "C:\\Users\\me"), ("LOCALAPPDATA", "C:\\Users\\me\\AppData\\Local")];
    let dirs = resolve(&env(&e), true);
    assert_eq!(dirs, vec![p("C:\\Users\\me\\AppData\\Local", "vtok"), p("C:\\Users\\me", ".local/state/vtok")]);
}

#[test]
fn a_process_without_home_sees_the_same_first_directory() {
    let with = [("HOME", "C:\\Users\\me"), ("LOCALAPPDATA", "C:\\Users\\me\\AppData\\Local")];
    let without = [("USERPROFILE", "C:\\Users\\me"), ("LOCALAPPDATA", "C:\\Users\\me\\AppData\\Local")];
    assert_eq!(resolve(&env(&with), true)[0], resolve(&env(&without), true)[0]);
}

#[test]
fn unix_uses_home_and_ignores_local_app_data() {
    let e = [("HOME", "/home/me"), ("LOCALAPPDATA", "/ignored")];
    assert_eq!(resolve(&env(&e), false), vec![p("/home/me", ".local/state/vtok")]);
}

#[test]
fn nothing_set_means_no_directory() {
    assert!(resolve(&env(&[]), true).is_empty());
    assert!(resolve(&env(&[]), false).is_empty());
}

#[test]
fn home_and_userprofile_naming_one_place_count_once() {
    let e = [("HOME", "/u/me"), ("USERPROFILE", "/u/me")];
    assert_eq!(resolve(&env(&e), false).len(), 1);
}
