//! A whole file: prose shrinks, code and structure stay, a second pass changes nothing.
use super::document::squeeze;

#[test]
fn prose_shrinks_but_code_and_structure_stay() {
    let src = "# Title  \n\n\n\nPlease note that you **must** run `make  sure to  go` in order to build.\n\n---\n\n```sh\n  just   run   this   \n```\n\n| a | b |\n|---|---|\n\n- Basically  keep   it simple, see https://x.y/z?a=1\n";
    let out = squeeze(src);
    assert!(out.contains("You must run `make  sure to  go` to build."), "{out}");
    assert!(out.contains("```sh\n  just   run   this\n```"), "code block kept (only trailing blanks trimmed): {out}");
    assert!(out.contains("| a | b |\n|---|---|"), "{out}");
    assert!(out.contains("- Keep it simple, see https://x.y/z?a=1"), "{out}");
    assert!(!out.contains("\n\n\n") && !out.contains("\n---\n"), "{out}");
    assert_eq!(squeeze(&out), out, "idempotent");
}
