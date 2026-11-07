//! What a JSON or YAML document holds besides the records the view is about: the other fields on the way down to them
//! (a page's `next` link or total, a second list), so the table is not taken for the whole document.
use super::records::clip;
use serde_json::Value;

/// `key=value`, `key[N]` or `key{a,b}` for every field beside the path from `root` to `holder`, the list or object that
/// holds the records. None when the records are the whole document.
pub fn around(root: &Value, holder: &Value) -> Option<String> {
    if std::ptr::eq(root, holder) { return None; }
    let mut out = vec![];
    walk(root, holder, "", &mut out);
    (!out.is_empty()).then(|| clip(&out.join(" "), 240))
}

fn walk(v: &Value, holder: &Value, path: &str, out: &mut Vec<String>) {
    let Value::Object(m) = v else { return };
    for (k, x) in m {
        if std::ptr::eq(x, holder) { continue; }
        let p = if path.is_empty() { k.clone() } else { format!("{path}.{k}") };
        if holds(x, holder, 0) { walk(x, holder, &p, out); } else { out.push(brief(&p, x)); }
    }
}

/// Whether `holder` is inside `v`, as deep as the search for records goes.
fn holds(v: &Value, holder: &Value, depth: usize) -> bool {
    match v {
        Value::Object(m) if depth < 3 => m.values().any(|x| std::ptr::eq(x, holder) || holds(x, holder, depth + 1)),
        _ => false,
    }
}

fn brief(path: &str, v: &Value) -> String {
    match v {
        Value::Array(a) => format!("{path}[{}]", a.len()),
        Value::Object(m) => {
            let keys: Vec<&str> = m.keys().take(5).map(String::as_str).collect();
            format!("{path}{{{}{}}}", keys.join(","), if m.len() > 5 { ",…" } else { "" })
        }
        Value::String(s) => format!("{path}={}", clip(s, 60)),
        other => format!("{path}={other}"),
    }
}
