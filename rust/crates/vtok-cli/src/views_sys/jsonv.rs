//! JSON structure summaries: keys, types and array lengths instead of values (or short values for API replies).
use super::util::*;
use serde_json::Value;

/// `values`: show short scalar values (API replies); otherwise only types.
pub fn summarize(v: &Value, values: bool, max_lines: usize) -> String {
    let mut out = vec![];
    walk(v, 0, "", values, &mut out);
    let n = out.len();
    out.truncate(max_lines);
    let mut s = out.join("\n");
    if n > max_lines { s.push_str(&format!("\n... +{} more lines", n - max_lines)); }
    s + "\n"
}

fn scalar(v: &Value, values: bool) -> String {
    match v {
        Value::Null => "null".into(),
        Value::Bool(b) => if values { b.to_string() } else { "bool".into() },
        Value::Number(n) => if values { n.to_string() } else { "num".into() },
        Value::String(s) => if values { format!("{:?}", trunc(s, 48)) } else { format!("str({})", s.chars().count()) },
        _ => String::new(),
    }
}

fn walk(v: &Value, depth: usize, key: &str, values: bool, out: &mut Vec<String>) {
    let pad = "  ".repeat(depth);
    let label = if key.is_empty() { String::new() } else { format!("{key}: ") };
    match v {
        Value::Object(m) if depth >= 5 => out.push(format!("{pad}{label}{{{} keys}}", m.len())),
        Value::Object(m) => {
            if m.is_empty() { out.push(format!("{pad}{label}{{}}")); return; }
            out.push(format!("{pad}{label}{{"));
            for (i, (k, c)) in m.iter().enumerate() {
                if i >= 25 { out.push(format!("{pad}  ... +{} keys", m.len() - 25)); break; }
                walk(c, depth + 1, k, values, out);
            }
            out.push(format!("{pad}}}"));
        }
        Value::Array(a) if a.is_empty() => out.push(format!("{pad}{label}[]")),
        Value::Array(a) => {
            let objs = a.iter().all(|x| x.is_object() || x.is_array());
            if depth >= 5 || (!objs && !values) || (!objs && a.len() > 6) {
                let kinds = if a.iter().all(|x| x.is_string()) { "str" } else if a.iter().all(|x| x.is_number()) { "num" } else { "mixed" };
                out.push(format!("{pad}{label}[{}] of {kinds}", a.len()));
                return;
            }
            out.push(format!("{pad}{label}[{}] first:", a.len()));
            walk(&a[0], depth + 1, "", values, out);
            if values && a.len() > 1 && objs { out.push(format!("{pad}  ... +{} more items", a.len() - 1)); }
        }
        _ => out.push(format!("{pad}{label}{}", scalar(v, values))),
    }
}

pub fn json_text(lines: &[String]) -> Option<String> {
    let text = lines.join("\n");
    let v: Value = serde_json::from_str(text.trim()).ok()?;
    if lines.len() < 8 { return None; }
    Some(summarize(&v, false, 80))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn structure_only() {
        let raw = serde_json::json!({"name":"vtok","version":"1.0.0","private":true,"deps":{"a":"1","b":"2"},
            "files":["a","b","c"],"items":[{"id":1,"tags":["x"]},{"id":2,"tags":[]}],"empty":[]});
        let text = serde_json::to_string_pretty(&raw).unwrap();
        let lines: Vec<String> = text.lines().map(String::from).collect();
        let o = json_text(&lines).unwrap();
        assert!(o.contains("name: str(4)") && o.contains("private: bool") && o.contains("files: [3] of str"));
        assert!(o.contains("items: [2] first:") && o.contains("id: num"));
        assert!(!o.contains("1.0.0") && o.len() < text.len());
    }
    #[test]
    fn values_mode_keeps_short_scalars() {
        let v = serde_json::json!({"Account":"123456789012","Items":[{"n":1},{"n":2},{"n":3}]});
        let o = summarize(&v, true, 50);
        assert!(o.contains("Account: \"123456789012\"") && o.contains("+2 more items"));
    }
    #[test]
    fn non_json_is_none() { assert!(json_text(&vec!["not json".to_string(); 10]).is_none()); }
}
