//! `vtok filter [--explain] [--code N] -- COMMAND...`: read a command's output from stdin and print what `vtok run` would
//! show for it, without running anything or storing anything (the id shown is a placeholder). `--code N` is the exit code
//! to pretend (default 0): a failure gets an exit line, a success may print nothing but the summary. `--explain` adds one
//! stderr line naming the stage that handled it (dedicated view, declarative TOML filter, shape-based view or the generic
//! summariser) and the token counts. For testing filters.
use std::io::Read;
use vtok_api::ntok;

pub fn run(args: &[String]) -> i32 {
    let explain = args.iter().any(|a| a == "--explain");
    let mut code = 0;
    let mut cmd: Vec<&str> = vec![];
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--" => { cmd.extend(args[i + 1..].iter().map(|s| s.as_str())); break; }
            "--code" => { i += 1; code = args.get(i).and_then(|s| s.parse().ok()).unwrap_or(0); }
            a if a.starts_with("--") => {}
            a => cmd.push(a),
        }
        i += 1;
    }
    if cmd.is_empty() { eprintln!("usage: vtok filter [--explain] [--code N] -- COMMAND...   (output on stdin)"); return 2; }
    let cmd = cmd.join(" ");
    let mut raw = String::new();
    if std::io::stdin().read_to_string(&mut raw).is_err() { return 1; }
    let lines = crate::shell::clean(&raw);
    let via = if crate::cmdviews::view(&cmd, &lines).is_some() { "dedicated" }
        else if crate::tomlfilters::apply(&cmd, &lines).is_some() { "toml" }
        else if crate::filters::view(&lines).is_some() { "shape" } else { "generic" };
    let out = crate::shell::preview(&cmd, &raw, code, 700);
    if explain { eprintln!("via={via} raw={} shown={}", ntok(&raw), ntok(&out)); }
    print!("{out}");
    0
}
