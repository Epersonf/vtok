use super::filter::{parse_duration, Filter};
use super::rec::Rec;

fn rec(t: u64, session: &str, cwd: &str) -> Rec {
    Rec { t, kind: "run".into(), raw: 100, shown: 10, session: session.into(), cwd: cwd.into(), tagged: true, ..Default::default() }
}

#[test]
fn durations_take_a_unit_suffix_or_plain_seconds() {
    assert_eq!(parse_duration("90s"), Some(90));
    assert_eq!(parse_duration("30m"), Some(1800));
    assert_eq!(parse_duration("2h"), Some(7200));
    assert_eq!(parse_duration("7d"), Some(604_800));
    assert_eq!(parse_duration("45"), Some(45));
    assert_eq!(parse_duration("soon"), None);
    assert_eq!(parse_duration(""), None);
}

#[test]
fn flags_set_the_filter_and_the_default_applies_only_when_nothing_narrows() {
    let a = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    assert_eq!(Filter::from_args(&a(&[]), Some(30)).unwrap().days, Some(30));
    assert_eq!(Filter::from_args(&a(&["--days", "7"]), Some(30)).unwrap().days, Some(7));
    let f = Filter::from_args(&a(&["--session", "abc", "--legacy"]), Some(30)).unwrap();
    assert_eq!((f.session.as_deref(), f.days, f.legacy), (Some("abc"), None, true), "a chat view is not limited to 30 days");
    assert_eq!(Filter::from_args(&a(&["--since", "2h"]), Some(30)).unwrap().since, Some(7200));
    assert!(Filter::from_args(&a(&["--days", "x"]), None).is_err());
    assert!(Filter::from_args(&a(&["--since", "x"]), None).is_err());
}

#[test]
fn json_arguments_follow_the_same_rules() {
    let f = Filter::from_json(&serde_json::json!({"session": "abc", "legacy": true, "cwd": "/p"}), Some(30)).unwrap();
    assert_eq!((f.session.as_deref(), f.cwd.as_deref(), f.days, f.legacy), (Some("abc"), Some("/p"), None, true));
    assert_eq!(Filter::from_json(&serde_json::json!({}), Some(30)).unwrap().days, Some(30));
}

#[test]
fn untagged_records_are_left_out_unless_legacy_is_asked() {
    let old = Rec { tagged: false, ..rec(1000, "", "") };
    let f = Filter::default();
    assert!(!f.matches(&old, 2000));
    assert!(Filter { legacy: true, ..f }.matches(&old, 2000));
}

#[test]
fn time_session_and_directory_narrow_the_view() {
    let now = 1_000_000;
    let r = rec(now - 3600, "chat-1", "C:\\Work\\app\\src");
    assert!(Filter { days: Some(1), ..Default::default() }.matches(&r, now));
    assert!(!Filter { since: Some(60), ..Default::default() }.matches(&r, now));
    assert!(Filter { session: Some("chat-1".into()), ..Default::default() }.matches(&r, now));
    assert!(!Filter { session: Some("chat-2".into()), ..Default::default() }.matches(&r, now));
    assert!(Filter { cwd: Some("C:/Work/app".into()), ..Default::default() }.matches(&r, now), "a subdirectory is inside");
    assert!(!Filter { cwd: Some("C:/Work/ap".into()), ..Default::default() }.matches(&r, now), "a longer name is not a parent");
    assert!(Filter { from: Some(now - 4000), to: Some(now - 3000), ..Default::default() }.matches(&r, now));
    assert!(!Filter { from: Some(now - 1000), ..Default::default() }.matches(&r, now));
}
