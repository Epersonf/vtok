//! YAML: repeated blocks, sequences of mappings, flow collections, comments and quotes; unsupported syntax gives up.
use super::yaml::read;
use super::view;

/// `n` services, service `odd` runs a release candidate.
pub(super) fn services(n: usize, odd: usize) -> Vec<String> {
    let mut v = vec!["# generated".to_string(), "services:".to_string()];
    for i in 0..n {
        let tag = if i == odd { "1.9.0-rc2" } else { "1.8.4" };
        for l in [format!("  svc-{i:02}:"), format!("    image: registry.local/svc-{i:02}:{tag}"), "    replicas: 3".into(),
                  "    limits: {cpu: 500m, memory: 512Mi}  # per pod".into(), "    note: \"a # b\"".into(), "    ports:".into(), "      - 80".into(),
                  "      - 443".into(), "    env:".into(), "      - name: LOG_LEVEL".into(), "        value: info".into()] { v.push(l); }
    }
    v
}

#[test]
fn the_service_with_another_image_tag_is_named() {
    let v = view(&services(45, 31), Some("yaml")).unwrap();
    assert!(v.starts_with("yaml: services 45 records, 8 fields\n"), "{v}");
    assert!(v.contains("svc-31: image=registry.local/svc-31:1.9.0-rc2 (unlike the rest, 1 of 45)"), "{v}");
    assert!(v.contains("limits.cpu=500m") && v.contains("note=a # b") && v.contains("ports=80,443"), "{v}");
}

#[test]
fn a_sequence_of_mappings_is_a_list_of_records() {
    let mut y = vec!["items:".to_string()];
    for i in 0..30 {
        y.push(format!("  - id: {i}"));
        y.push(format!("    kind: {}", if i == 12 { "odd" } else { "plain" }));
        y.push("    tags: [a, b]".to_string());
    }
    let v = view(&y, Some("yml")).unwrap();
    assert!(v.starts_with("yaml: items 30 records, 3 fields"), "{v}");
    assert!(v.contains("items[12]: kind=odd (1 of 30)"), "{v}");
}

#[test]
fn scalars_get_their_types_and_text_blocks_are_text() {
    let y: Vec<String> = "a: 1\nb: 'x y'\nc: true\nd: ~\ne: 007\nf: |\n  line one\n  line two\ng: [1, \"two\", {k: v}]".lines().map(String::from).collect();
    let v = read(&y);
    assert!(v.is_none(), "one mapping is not a list of records");
    let t = super::yaml::parse(&y).unwrap();
    assert_eq!(t["a"], 1);
    assert_eq!(t["b"], "x y");
    assert_eq!(t["c"], true);
    assert!(t["d"].is_null());
    assert_eq!(t["e"], "007");
    assert_eq!(t["f"], "line one\nline two");
    assert_eq!(t["g"][2]["k"], "v");
}

#[test]
fn syntax_the_reader_does_not_take_on_is_refused() {
    let mut y = services(30, 3);
    y.insert(2, "base: &anchor".to_string());
    assert!(view(&y, Some("yaml")).is_none(), "anchors");
    let mut y = services(30, 3);
    y.push("---".to_string());
    assert!(view(&y, Some("yaml")).is_none(), "second document");
    let mut y = services(30, 3);
    y.push("\tbad: tab".to_string());
    assert!(view(&y, Some("yaml")).is_none(), "tabs");
    let mut y = services(30, 3);
    y.push("  svc-00: again".to_string());
    assert!(view(&y, Some("yaml")).is_none(), "duplicate key");
}
