//! `vtok find`: words of a question ranked against symbol names and paths.
use super::search::{find_symbols, name_tokens};

#[test]
fn find_ranks_by_name_words_and_path() {
    let d = std::env::temp_dir().join(format!("vtok-find-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("cookies.py"), "class CookieJar:\n    def get(self):\n        return 1\n\ndef cookiejar_from_dict(d):\n    return d\n").unwrap();
    std::fs::write(d.join("auth.py"), "def basic_auth_str(u):\n    return u\n\ndef unrelated():\n    return 2\n").unwrap();
    let r = find_symbols("where is the cookie jar built from a dict", &d, 300).text;
    let first = r.lines().nth(1).unwrap();
    assert!(first.contains("cookiejar_from_dict") || first.contains("CookieJar"), "{r}");
    assert!(!r.contains("unrelated"), "{r}");
    let a = find_symbols("basic auth", &d, 300).text;
    assert!(a.lines().nth(1).unwrap().contains("basic_auth_str"), "{a}");
    assert!(find_symbols("zzzz", &d, 300).text.contains("nothing matches"));
    assert_eq!(name_tokens("fooBar_baz2"), vec!["foo", "bar", "baz2"]);
    std::fs::remove_dir_all(&d).ok();
}
