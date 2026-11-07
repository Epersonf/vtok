//! The closing-line policy: nothing for verbatim output or a fully summarised success; `# more:` when the view cut
//! content; `# exit=N;` plus the id when the command failed.
use super::compact::{build, preview, Kind};
use super::omitted::admits_omission;
use vtok_api::ntok;

const ID: &str = "00000000";

fn noisy(errors: bool) -> String {
    let mut s = String::new();
    for i in 0..300 { s += &format!("step {i} of the build finished without incident\n"); }
    if errors { s += "error[E0308]: mismatched types\n  --> src/lib.rs:10:5\n"; }
    s += "test result: ok. 3 passed; 0 failed; 0 ignored\n";
    s
}

#[test]
fn short_output_is_verbatim_with_no_footer_even_on_failure() {
    assert_eq!(preview("false", "boom\n", 1, 700), "boom\n");
    assert_eq!(preview("true", "", 0, 700), "");
}

#[test]
fn failed_run_with_a_cut_view_gets_exit_and_id() {
    let out = preview("./build.sh", &noisy(true), 2, 700);
    assert!(out.ends_with(&format!("# exit=2; more: vtok output {ID}\n")), "{out}");
    assert!(out.contains("mismatched types"), "{out}");
    assert_eq!(out.matches("vtok output").count(), 1, "one closing line, no header: {out}");
    assert!(!out.starts_with('#'), "{out}");
}

#[test]
fn successful_run_with_a_cut_view_gets_only_the_more_line() {
    let c = build("./build.sh", &noisy(false), 0, 700, false);
    assert_ne!(c.kind, Kind::Verbatim);
    let out = c.render(0);
    assert!(out.ends_with(&format!("# more: vtok output {ID}\n")) && !out.contains("exit="), "{out}");
}

#[test]
fn mcp_style_footer_leaves_the_exit_status_to_the_caller() {
    let c = build("./build.sh", &noisy(true), 2, 700, false);
    assert_eq!(c.footer(2, false).as_deref(), Some(format!("# more: vtok output {ID}\n").as_str()));
    assert_eq!(c.footer(2, true).as_deref(), Some(format!("# exit=2; more: vtok output {ID}\n").as_str()));
    let v = build("x", "ok\n", 3, 700, false);
    assert!(v.kind == Kind::Verbatim && v.footer(3, true).is_none());
}

#[test]
fn a_view_that_is_not_smaller_is_replaced_by_the_output() {
    // every line is an error with its own words, so the summariser keeps all of them and adds a closing line
    let words = ["alpha", "bravo", "charlie", "delta", "echo", "foxtrot", "golf", "hotel", "india", "juliet", "kilo", "lima"];
    let raw: String = (0..60).map(|i| format!("error: {} {} {}\n", words[i % 12], words[(i * 5 + 1) % 12], words[(i * 7 + 3) % 12])).collect();
    assert_eq!(preview("./build.sh", &raw, 1, 5000), raw);
}

#[test]
fn views_admit_cuts_in_their_own_words() {
    for yes in ["a\n... +4 more\n", "x +12 more files\n", "… 3 lines omitted …\n", "L5:\nfoo  [×3 similar]\n", "7 entries hidden\n", "+2 more kinds"] {
        assert!(admits_omission(yes), "{yes}");
    }
    for no in ["TS2322 x34: Type 'string' is not assignable\n  a.ts: 1, 2, 4\n", "cargo test: 166 passed (3 suites), 4 warnings\n", "12 more tests passed\n", "x3 y+3\n"] {
        assert!(!admits_omission(no), "{no}");
    }
}

#[test]
fn cut_views_are_much_smaller_than_the_output() {
    let raw = noisy(true);
    let c = build("./build.sh", &raw, 2, 700, false);
    assert!(ntok(&c.render(2)) < ntok(&raw) / 4);
}
