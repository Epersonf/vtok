//! Command-aware dedicated views, tried before the shape-based ones. Each `views_*` module exposes
//! `view(cmd, lines) -> Option<String>`: `cmd` is the command line as typed (it decides which tool this is),
//! `lines` the cleaned output. Return None when the output is not what that tool prints, so the generic summariser runs.
pub fn view(cmd: &str, lines: &[String]) -> Option<String> {
    if cmd.is_empty() { return None; }
    crate::views_lang::view(cmd, lines).or_else(|| crate::views_build::view(cmd, lines)).or_else(|| crate::views_sys::view(cmd, lines))
}
