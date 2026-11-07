//! Name lists for resolution: method names that standard libraries define everywhere (a call `x.find()` with an
//! unknown receiver is far more likely `str::find` than the one workspace method named `find`), and test detection.

/// Methods of the standard types of Rust, Python, JavaScript/TypeScript, Go and Java (strings, collections,
/// iterators, options/results, paths, IO, maps, promises) plus the most common trait methods.
pub(crate) const COMMON_METHODS: &[&str] = &[
    "new", "default", "len", "is_empty", "get", "get_mut", "set", "push", "push_str", "pop", "insert", "remove", "contains", "contains_key",
    "clear", "extend", "append", "truncate", "retain", "drain", "sort", "sort_by", "sort_by_key", "dedup", "reverse", "swap", "split_off",
    "iter", "iter_mut", "into_iter", "map", "filter", "filter_map", "flat_map", "find", "find_map", "position", "any", "all", "count", "sum",
    "min", "max", "min_by_key", "max_by_key", "fold", "collect", "chain", "zip", "rev", "skip", "take", "take_while", "skip_while", "step_by",
    "enumerate", "peekable", "next", "last", "first", "nth", "cloned", "copied", "flatten", "partition", "for_each", "inspect", "windows", "chunks",
    "lines", "chars", "bytes", "split", "splitn", "rsplit", "rsplitn", "split_once", "rsplit_once", "split_whitespace", "trim", "trim_start",
    "trim_end", "trim_matches", "trim_start_matches", "trim_end_matches", "strip_prefix", "strip_suffix", "starts_with", "ends_with",
    "replace", "replacen", "to_lowercase", "to_uppercase", "to_ascii_lowercase", "to_ascii_uppercase", "repeat", "parse", "matches", "rfind",
    "char_indices", "as_str", "as_bytes", "to_string", "to_owned", "into", "from", "as_ref", "as_mut", "borrow", "borrow_mut", "clone",
    "unwrap", "unwrap_or", "unwrap_or_else", "unwrap_or_default", "expect", "ok", "err", "ok_or", "ok_or_else", "and_then", "or_else",
    "map_err", "map_or", "map_or_else", "is_some", "is_none", "is_ok", "is_err", "filter", "flatten", "take", "replace", "as_deref",
    "entry", "or_insert", "or_insert_with", "or_default", "keys", "values", "values_mut", "items", "update", "join", "format", "write",
    "write_all", "writeln", "read", "read_to_string", "read_line", "flush", "open", "close", "create", "exists", "is_dir", "is_file",
    "metadata", "file_name", "file_stem", "extension", "parent", "path", "display", "to_path_buf", "canonicalize", "with_extension",
    "strip_prefix", "components", "modified", "elapsed", "now", "lock", "send", "recv", "spawn", "wait", "kill", "output", "status", "arg",
    "args", "env", "current_dir", "stdout", "stderr", "stdin", "fmt", "eq", "ne", "cmp", "partial_cmp", "hash", "drop", "deref", "index",
    "add", "sub", "mul", "div", "neg", "not", "then", "then_some", "abs", "pow", "sqrt", "floor", "ceil", "round", "saturating_sub",
    "checked_sub", "checked_add", "wrapping_add", "to_vec", "capacity", "with_capacity", "reserve", "resize", "fill", "binary_search",
    "is_match", "captures", "captures_iter", "find_iter", "replace_all", "get_or_init", "as_array", "as_object", "as_u64", "as_i64",
    "as_f64", "as_bool", "kind", "start", "end", "child", "children", "walk", "parent", "text", "range", "name",
    // Python / JavaScript / TypeScript / Go / Java
    "append", "add", "has", "delete", "forEach", "indexOf", "includes", "slice", "splice", "concat", "push", "shift", "unshift", "some",
    "every", "reduce", "toString", "valueOf", "hasOwnProperty", "length", "size", "strip", "lstrip", "rstrip", "lower", "upper", "encode",
    "decode", "startswith", "endswith", "setdefault", "copy", "sorted", "group", "groups", "search", "match", "test", "exec", "then",
    "catch", "finally", "resolve", "reject", "log", "error", "warn", "info", "debug", "equals", "hashCode", "compareTo", "close", "run",
    "call", "apply", "bind", "keys", "entries", "assign", "freeze", "stringify", "dumps", "loads", "load", "dump", "emit", "on", "off",
];

pub(crate) fn common_method(n: &str) -> bool { COMMON_METHODS.contains(&n) }

/// A symbol is test code when its file is a test file or it sits in a `tests` module / test class, or is named test*.
pub(crate) fn is_test(file: &str, name: &str, attr_above: &str) -> bool {
    let base = file.rsplit('/').next().unwrap_or(file);
    let stem = base.split('.').next().unwrap_or(base);
    file.split('/').any(|d| d == "tests" || d == "test" || d == "__tests__")
        || stem.starts_with("test_") || stem.ends_with("_test") || stem.ends_with("_tests") || stem.starts_with("tests_") || stem == "tests"
        || base.contains(".test.") || base.contains(".spec.")
        || name.split('.').any(|s| s == "tests" || s == "test" || s.starts_with("test_") || s.starts_with("Test"))
        || attr_above.contains("#[test]") || attr_above.contains("#[cfg(test)]")
}
