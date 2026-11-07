//! JSON: a list of objects inside a document, a bare list, newline-delimited JSON, and documents that are not lists.
use super::view;
use serde_json::json;

pub(super) fn lines(s: &str) -> Vec<String> { s.lines().map(String::from).collect() }

/// A Kubernetes-style list of `n` deployments; deployment `odd` has a CPU limit eight times the others.
pub(super) fn deployments(n: usize, odd: usize) -> serde_json::Value {
    let items: Vec<_> = (0..n).map(|i| json!({"name": format!("deploy-{i:03}"), "replicas": 3, "labels": {"team": "platform"},
        "resources": {"limits": {"cpu": if i == odd { "4000m" } else { "500m" }}}})).collect();
    json!({"kind": "List", "items": items})
}

#[test]
fn the_drifted_value_in_a_list_of_objects_is_named() {
    let doc = serde_json::to_string_pretty(&deployments(130, 88)).unwrap();
    let v = view(&lines(&doc), Some("json")).unwrap();
    assert!(v.starts_with("json: items 130 records, 4 fields\n"), "{v}");
    assert!(v.contains("items[88] name=deploy-088: resources.limits.cpu=4000m (1 of 130)"), "{v}");
    assert!(v.contains("labels.team=platform") && v.contains("replicas=3") && v.contains("name: unique, deploy-000 … deploy-129"), "{v}");
    assert!(v.contains("first (items[0]): labels.team=platform name=deploy-000"), "{v}");
}

#[test]
fn a_bare_list_and_a_map_of_objects_are_records_too() {
    let list = serde_json::to_string_pretty(&deployments(40, 7)["items"]).unwrap();
    let v = view(&lines(&list), Some("json")).unwrap();
    assert!(v.starts_with("json: 40 records") && v.contains("[7] name=deploy-007"), "{v}");
    let map: serde_json::Map<String, serde_json::Value> = (0..40).map(|i| (format!("host{i:02}"), json!({"up": i != 9, "load": 1 + i % 3}))).collect();
    let v = view(&lines(&serde_json::to_string_pretty(&map).unwrap()), Some("json")).unwrap();
    assert!(v.contains("host09: up=false (1 of 40)"), "{v}");
}

#[test]
fn newline_delimited_json_keeps_the_event_line() {
    let ev: Vec<String> = (0..200).map(|i| json!({"ts": 1000 + i, "level": if i == 149 { "ERROR" } else { "INFO" }, "ms": 20 + i % 20}).to_string()).collect();
    let v = view(&ev, Some("ndjson")).unwrap();
    assert!(v.starts_with("ndjson: 200 rows") && v.contains("L150: {\"level\":\"ERROR\""), "{v}");
    assert!(v.contains("level=ERROR (1 of 200)") && v.contains("level: INFO 199, ERROR 1"), "{v}");
    // the same lines under a name that says nothing are recognised from the text
    assert!(view(&ev, None).unwrap().starts_with("ndjson: 200 rows"));
}

#[test]
fn a_minified_document_on_one_line_is_a_table_too() {
    let one = serde_json::to_string(&deployments(900, 640)["items"]).unwrap();
    assert!(!one.contains('\n'));
    let v = view(&[one.clone()], Some("json")).unwrap();
    assert!(v.starts_with("json: 900 records") && v.contains("[640] name=deploy-640: resources.limits.cpu=4000m (1 of 900)"), "{v}");
    // and the same document is recognised from its first character when the command names no file
    assert!(view(&[one], None).unwrap().starts_with("json: 900 records"));
}

#[test]
fn the_other_fields_of_the_document_stay_in_view() {
    let items: Vec<_> = (0..60).map(|i| json!({"id": i, "state": if i == 41 { "failed" } else { "ok" }, "ms": 20 + i % 9})).collect();
    let page = json!({"count": 18000, "next": "https://api.example.com/v1/jobs?page=2", "meta": {"took": 3, "shards": 5}, "results": items, "history": [1, 2, 3]});
    let v = view(&[page.to_string()], Some("json")).unwrap();
    assert!(v.starts_with("json: results 60 records, 3 fields\nalso: "), "{v}");
    for part in ["count=18000", "next=https://api.example.com/v1/jobs?page=2", "meta{shards,took}", "history[3]"] { assert!(v.contains(part), "{part} in {v}"); }
    // with two lists the larger is the table and the other is named
    let rec = |i: usize| json!({"src": format!("n{}", i % 30), "dst": format!("n{}", (i * 7) % 30), "kind": "calls"});
    let graph = json!({"version": 3, "nodes": (0..30).map(rec).collect::<Vec<_>>(), "edges": (0..200).map(rec).collect::<Vec<_>>()});
    let v = view(&lines(&serde_json::to_string_pretty(&graph).unwrap()), Some("json")).unwrap();
    assert!(v.starts_with("json: edges 200 records") && v.contains("also: nodes[30] version=3"), "{v}");
    // records that are the whole document have nothing beside them
    assert!(!view(&lines(&serde_json::to_string_pretty(&deployments(40, 3)["items"]).unwrap()), Some("json")).unwrap().contains("also:"));
}

#[test]
fn objects_that_share_no_fields_are_parts_of_a_document_not_rows() {
    // the steps of a workflow, each with keys of its own
    let keys = ["uses", "run", "with", "env", "if", "id", "name", "shell", "timeout"];
    let steps: Vec<_> = (0..24).map(|i| json!({keys[i % 9]: format!("v{i}"), keys[(i * 5 + 1) % 9]: i})).collect();
    let doc = serde_json::to_string_pretty(&json!({"jobs": {"build": {"steps": steps}}})).unwrap();
    assert!(view(&lines(&doc), Some("json")).is_none());
    let yaml: Vec<String> = ["steps:"].into_iter().map(String::from).chain((0..24).flat_map(|i| match i % 3 {
        0 => vec![format!("  - uses: actions/step{i}")],
        1 => vec![format!("  - name: Build {i}"), format!("    run: make {i}")],
        _ => vec![format!("  - if: always{i}"), format!("    id: s{i}")],
    })).collect();
    assert!(view(&yaml, Some("yml")).is_none(), "{yaml:?}");
}

#[test]
fn documents_that_are_not_lists_of_objects_are_left_alone() {
    let config: serde_json::Map<String, serde_json::Value> = (0..40).map(|i| (format!("key{i}"), json!(i))).collect();
    assert!(view(&lines(&serde_json::to_string_pretty(&config).unwrap()), Some("json")).is_none());
    assert!(view(&lines(&serde_json::to_string_pretty(&json!({"ids": (0..100).collect::<Vec<_>>()})).unwrap()), Some("json")).is_none());
    assert!(view(&lines("{ not json at all"), Some("json")).is_none());
}

#[test]
fn nested_lists_and_missing_fields_are_flattened_and_noticed() {
    let items: Vec<_> = (0..60).map(|i| {
        let mut o = json!({"id": i, "tags": ["a", "b"], "owner": {"name": "ops"}, "ports": [{"n": 80}, {"n": 443}]});
        if i == 33 { o["debug"] = json!(true); }
        if i == 41 { o.as_object_mut().unwrap().remove("owner"); }
        o
    }).collect();
    let v = view(&lines(&serde_json::to_string_pretty(&items).unwrap()), Some("json")).unwrap();
    assert!(v.contains("ports[0].n=80") && v.contains("tags=a,b"), "{v}");
    assert!(v.contains("[33]") && v.contains("debug=true (only 1 of 60 have it)"), "{v}");
    assert!(v.contains("[41]") && v.contains("no owner.name (59 of 60 have it)"), "{v}");
}
