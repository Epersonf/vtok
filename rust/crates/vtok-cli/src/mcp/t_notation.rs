use super::describe::instructions;
use super::notation::first_use;
use super::source::{tool_chunk, tool_read};
use serde_json::json;
use vtok_api::ntok;

/// A C# file whose comments are long enough for the compact form to pay at any min-gain.
fn csharp_file(tag: &str) -> String {
    let dir = std::env::temp_dir().join(format!("vtok-notation-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let methods: String = (0..12).map(|i| format!("        // compute value number {i} very carefully, step by step, with a long explanatory comment on top\n        public int Compute{i}(int first, int second) {{\n            int total = first + second * {i};\n            return total;\n        }}\n")).collect();
    let path = dir.join("Calc.cs");
    std::fs::write(&path, format!("using System;\n\nnamespace Demo {{\n    public class Calculator {{\n{methods}    }}\n}}\n")).unwrap();
    path.to_string_lossy().into_owned()
}

#[test]
fn the_server_instructions_carry_no_notation_rules() {
    let s = instructions();
    assert!(ntok(&s) < 500, "{} tokens", ntok(&s));
    assert!(!s.contains("Vtok-Rust") && !s.contains("[python]"), "{s}");
    for needle in ["vtok_outline", "vtok_find", "vtok_chunk", "vtok_edit", "replace", "vtok run", "vtok_output"] {
        assert!(s.contains(needle), "{needle} missing: {s}");
    }
}

#[test]
fn the_rules_of_a_language_come_once() {
    let first = first_use("dart");
    assert!(first.starts_with("# notation of dart\n") && first.contains("Vtok-"), "{first}");
    assert_eq!(first_use("dart"), "");
    assert_eq!(first_use("no-such-language"), "");
}

#[test]
fn the_first_compact_read_and_chunk_of_a_language_carry_its_rules_and_later_ones_do_not() {
    let path = csharp_file("read");
    let first = tool_read(&json!({"path": path})).unwrap();
    assert!(!first.contains("plain text"), "the compact form should pay here: {first}");
    assert!(first.starts_with("# notation of csharp\n"), "{first}");
    let again = tool_read(&json!({"path": path})).unwrap();
    assert!(again.starts_with("# ") && !again.contains("# notation of"), "{again}");
    let chunk = tool_chunk(&json!({"path": path, "id": "Demo"})).unwrap();
    assert!(!chunk.contains("# notation of"), "{chunk}");
}
