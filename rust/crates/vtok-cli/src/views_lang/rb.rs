//! rspec, rubocop and minitest (rake test / rails test): failures with locations, offenses, summaries.
use super::util::*;
use crate::diag::cap_classes;

pub fn rspec(lines: &[String]) -> Option<String> {
    if !has(lines, r"^\d+ examples?, \d+ failures?|^No examples found|error occurred while loading") { return None; }
    let from = lines.iter().position(|l| l.trim() == "Failures:" || l.contains("An error occurred while loading")).unwrap_or(lines.len());
    let numbered = re(r"^\s+\d+\) ");
    let start = |l: &str| numbered.is_match(l) || l.contains("An error occurred while loading");
    let stop = |l: &str| l.starts_with("Finished in") || l.starts_with("Failed examples") || l.trim() == "Pending:";
    // keep the first project backtrace line only; gem frames are noise
    let drop = |l: &str| { let t = l.trim_start(); t.starts_with("# ") && (t.contains("/gems/") || t.contains("/lib/ruby/") || t.contains("<main>") || t.starts_with("# -e")) };
    let mut out = blocks(&lines[from.min(lines.len())..], &start, &stop, 14, &drop);
    out.extend(grep(lines, r"^(Finished in|\d+ examples?,)"));
    let failed = grep(lines, r"^rspec \S+");
    if !failed.is_empty() { out.push("Failed examples:".into()); out.extend(cap(failed, 30)); }
    finish(out)
}

pub fn rubocop(lines: &[String]) -> Option<String> {
    let off = re(r"^(\S+?:\d+:\d+): ([CWEFR]): (?:\[Correctable\] )?([\w/]+): (.*)$");
    let (mut out, mut cops): (Vec<(String, String)>, Vec<String>) = (vec![], vec![]);
    for l in lines {
        if let Some(c) = off.captures(l) {
            cops.push(c[3].to_string());
            out.push((c[3].to_string(), format!("{} {} {}: {}", &c[1], &c[2], &c[3], short(&c[4], 140))));
        }
    }
    let tail = grep(lines, r"files? inspected|no offenses detected");
    if out.is_empty() && tail.is_empty() { return None; }
    let mut v = cap_classes(out, 60);
    if cops.len() > 8 { v.push(format!("by cop: {}", short(&tally(&cops), 300))); }
    v.extend(tail);
    finish(v)
}

pub fn minitest(lines: &[String]) -> Option<String> {
    if !has(lines, r"^\d+ (runs|tests), \d+ assertions") { return None; }
    let start = |l: &str| re(r"^\s*\d+\) (Failure|Error):").is_match(l);
    let stop = |l: &str| re(r"^\d+ (runs|tests),").is_match(l);
    let drop = |l: &str| l.contains("/gems/") || l.contains("/lib/ruby/") || l.trim_start().starts_with("bin/rails");
    let mut out = blocks(lines, &start, &stop, 10, &drop);
    out.extend(grep(lines, r"^(Finished in|\d+ (runs|tests),)"));
    finish(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::util::t::*;

    #[test]
    fn rspec_failure_block() {
        let s = "Randomized with seed 4242\n.....F.*.....\n\nPending: (Failures listed here are expected and do not affect your suite's status)\n\n  1) Thing does stuff\n     # Not yet implemented\n     # ./spec/thing_spec.rb:3\n\nFailures:\n\n  1) User#name returns the name\n     Failure/Error: expect(user.name).to eq(\"x\")\n\n       expected: \"x\"\n            got: \"y\"\n\n       (compared using ==)\n     # ./spec/user_spec.rb:10:in `block (3 levels) in <top (required)>'\n     # /usr/lib/ruby/gems/3.2.0/gems/rspec-core-3.12/lib/rspec/core/example.rb:263:in `block in run'\n\nFinished in 0.5 seconds (files took 1.2 seconds to load)\n14 examples, 1 failure, 1 pending\n\nFailed examples:\n\nrspec ./spec/user_spec.rb:9 # User#name returns the name\n";
        let l = lines(s);
        let o = small(rspec(&l), &l);
        assert!(o.contains("User#name returns the name") && o.contains("expected: \"x\"") && o.contains("spec/user_spec.rb:10"));
        assert!(o.contains("14 examples, 1 failure") && o.contains("rspec ./spec/user_spec.rb:9"));
        assert!(!o.contains("Not yet implemented") && !o.contains("rspec-core") && !o.contains("Randomized"));
    }

    #[test]
    fn rubocop_offenses_and_tally() {
        let mut s = String::from("Inspecting 40 files\n..C..W....\n\nOffenses:\n\n");
        for i in 0..9 { s += &format!("app/m{i}.rb:{}:1: C: [Correctable] Style/FrozenStringLiteralComment: Missing frozen string literal comment.\n# frozen_string_literal: true\n^\n", i + 1); }
        s += "\n40 files inspected, 9 offenses detected, 9 offenses autocorrectable\n";
        let l = lines(&s);
        let o = small(rubocop(&l), &l);
        assert!(o.contains("app/m4.rb:5:1 C Style/FrozenStringLiteralComment") && o.contains("by cop: Style/FrozenStringLiteralComment x9"));
        assert!(o.contains("40 files inspected") && !o.contains("Inspecting") && !o.contains("^"));
    }

    #[test]
    fn minitest_failures() {
        let s = "Run options: --seed 1234\n\n# Running:\n\n..F.E\n\nFinished in 0.01s, 100.0 runs/s, 300 assertions/s.\n\n  1) Failure:\nUserTest#test_name [test/user_test.rb:10]:\nExpected: \"a\"\n  Actual: \"b\"\n\n  2) Error:\nOrderTest#test_total:\nNoMethodError: undefined method `x' for nil\n    app/models/order.rb:7:in `total'\n    /usr/lib/ruby/gems/3.2.0/gems/minitest/lib/minitest/test.rb:98:in `run'\n\n5 runs, 8 assertions, 1 failures, 1 errors, 0 skips\n";
        let l = lines(s);
        let o = small(minitest(&l), &l);
        assert!(o.contains("test/user_test.rb:10") && o.contains("NoMethodError") && o.contains("app/models/order.rb:7") && o.contains("5 runs, 8 assertions"));
        assert!(!o.contains("minitest/test.rb") && !o.contains("Run options"));
    }
}
