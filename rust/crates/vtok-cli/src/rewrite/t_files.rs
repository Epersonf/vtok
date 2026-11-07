//! A bare `cat` of a large data file is wrapped in `vtok run`; every other thing about files is left alone.
use super::*;
use std::path::PathBuf;

/// A scratch directory with 36,000-byte data files of several kinds and a small CSV.
fn scratch(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("vtok-rw-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    for name in ["big.csv", "big.json", "big.yaml", "big.txt", "BIG.CSV"] { std::fs::write(d.join(name), "a,b,c\n".repeat(6000)).unwrap(); }
    std::fs::write(d.join("small.csv"), "a,b,c\n".repeat(100)).unwrap();
    d
}

#[test]
fn a_plain_cat_of_a_big_data_file_is_wrapped() {
    let d = scratch("cat");
    let at = |cmd: &str| rewrite_in(cmd, false, d.to_str());
    assert_eq!(at("cat big.csv").as_deref(), Some("vtok run -- cat big.csv"));
    for f in ["big.json", "big.yaml", "BIG.CSV", "'big.csv'"] { assert!(at(&format!("cat {f}")).is_some(), "{f}"); }
    let abs = d.join("big.csv").display().to_string();
    if !abs.contains(char::is_whitespace) { assert_eq!(at(&format!("cat \"{abs}\"")), Some(format!("vtok run -- cat \"{abs}\""))); }
    std::fs::remove_dir_all(&d).ok();
}

#[test]
fn only_one_plain_big_data_file_qualifies() {
    let d = scratch("only");
    let at = |cmd: &str| rewrite_in(cmd, false, d.to_str());
    for no in ["cat small.csv", "cat big.txt", "cat missing.csv", "cat -n big.csv", "cat big.csv big.json", "cat big.csv | head", "cat big.csv > copy.csv", "cat ~/big.csv", "cat $F.csv", "cat", "cat ."] {
        assert!(at(no).is_none(), "{no}");
    }
    // relative names are the shell's: from another directory the same name is not found
    assert!(rewrite_in("cat big.csv", false, Some("/no/such/dir")).is_none());
    std::fs::remove_dir_all(&d).ok();
}

#[test]
fn a_cd_before_the_cat_moves_where_the_file_is_looked_for() {
    let d = scratch("cd");
    let (parent, name) = (d.parent().unwrap().to_str().unwrap().to_string(), d.file_name().unwrap().to_str().unwrap().to_string());
    assert_eq!(rewrite_in(&format!("cd {name} && cat big.csv"), false, Some(&parent)), Some(format!("cd {name} && vtok run -- cat big.csv")));
    assert_eq!(rewrite_in(&format!("cd {name}; cat small.csv"), false, Some(&parent)), None);
    for lost in ["cd - && cat big.csv", "cd ~ && cat big.csv", "cd $HOME && cat big.csv", "cd && cat big.csv"] { assert!(rewrite_in(lost, false, d.to_str()).is_none(), "{lost}"); }
    std::fs::remove_dir_all(&d).ok();
}

#[test]
fn hooks_that_imply_allow_never_get_the_cat_rewrite() {
    let d = scratch("allow");
    assert!(rewrite_in("cat big.csv", true, d.to_str()).is_none());
    std::fs::remove_dir_all(&d).ok();
}

#[test]
fn the_hook_reads_files_from_the_directory_its_input_names() {
    let d = scratch("hook");
    let input = |cwd: &str| serde_json::json!({"tool_name": "Bash", "cwd": cwd, "tool_input": {"command": "cat big.csv"}}).to_string();
    let out: serde_json::Value = serde_json::from_str(&crate::hook::answer(&input(d.to_str().unwrap())).unwrap()).unwrap();
    assert_eq!(out["hookSpecificOutput"]["updatedInput"]["command"], "vtok run -- cat big.csv");
    assert!(crate::hook::answer(&input("/no/such/dir")).is_none());
    std::fs::remove_dir_all(&d).ok();
}

#[cfg(windows)]
#[test]
fn git_bash_drive_paths_are_understood() {
    let d = scratch("gitbash");
    let s = d.display().to_string().replace('\\', "/");
    let msys = format!("/{}{}", s[..1].to_lowercase(), &s[2..]);
    if !msys.contains(char::is_whitespace) { assert!(rewrite_in(&format!("cat {msys}/big.csv"), false, Some("/")).is_some(), "{msys}"); }
    std::fs::remove_dir_all(&d).ok();
}
