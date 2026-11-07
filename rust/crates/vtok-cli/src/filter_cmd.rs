//! `vtok filter [--explain] -- COMMAND...`: read a command's output from stdin and print what `vtok run` would show for
//! it, without running anything. `--explain` adds one stderr line naming the stage that handled it (dedicated view,
//! declarative TOML filter, shape-based view or the generic summariser) and the token counts. For testing filters.
use std::io::Read;
use vtok_api::ntok;

pub fn run(args: &[String]) -> i32 {
    let explain = args.iter().any(|a| a == "--explain");
    let cmd: Vec<&String> = args.iter().filter(|a| !a.starts_with("--")).collect();
    if cmd.is_empty() { eprintln!("usage: vtok filter [--explain] -- COMMAND...   (output on stdin)"); return 2; }
    let cmd = cmd.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(" ");
    let mut raw = String::new();
    if std::io::stdin().read_to_string(&mut raw).is_err() { return 1; }
    let lines = crate::shell::clean(&raw);
    let via = if crate::cmdviews::view(&cmd, &lines).is_some() { "dedicated" }
        else if crate::tomlfilters::apply(&cmd, &lines).is_some() { "toml" }
        else if crate::filters::view(&lines).is_some() { "shape" } else { "generic" };
    let out = crate::shell::compact_cmd(&cmd, &raw, 0, 700);
    if explain { eprintln!("via={via} raw={} shown={}", ntok(&raw), ntok(&out)); }
    print!("{out}");
    0
}
