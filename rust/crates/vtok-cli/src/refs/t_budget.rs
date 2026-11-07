//! Non-ASCII names and large trees: no panic, and the output stays inside the token budget.
use super::overview::map;
use super::search::{find_symbols, word_match};
use vtok_api::ntok;

#[test]
fn non_ascii_names_do_not_panic_and_big_trees_stay_in_budget() {
    let d = std::env::temp_dir().join(format!("vtok-big-{}", std::process::id()));
    for i in 0..40 { std::fs::create_dir_all(d.join(format!("pkg{}/sub", i % 8))).unwrap(); std::fs::write(d.join(format!("pkg{}/sub/m{i}.py", i % 8)), format!("def f{i}():\n    return {i}\n\ndef ع_func{i}():\n    return f{i}()\n")).unwrap(); }
    assert!(find_symbols("عرض الحجم func", &d, 300).text.starts_with("# find"));
    assert_eq!(word_match("abcdéx", "abcdéyz"), 1.5);
    let m = map(&d, 200).text;
    assert!(ntok(&m) <= 260, "{} tokens: {m}", ntok(&m));
    assert!(m.contains("pkg"), "{m}");
    std::fs::remove_dir_all(&d).ok();
}
