//! The `vtok` command line. `run` dispatches on the first argument; most commands are a module of their own
//! (`shell` is `run`, `refs` is `refs`/`map`/`find`, `graph`, `hook`, `mcp`, ...). The notation commands are
//! `cmd_codec` and `cmd_chunks`; `args` reads flags, `help` has the usage text, `jsonmap` the tiny JSON the CLI needs.
mod agents;
mod auto;
mod browse;
mod config;
mod data;
mod diag;
mod discover;
mod filter_cmd;
mod filters;
mod filters_tools;
mod init;
mod gitfmt;
mod gitdiff;
mod graph;
mod graph_out;
mod hook;
mod html_text;
mod hook_agents;
mod hook_install;
mod hook_plugins;
mod risk;
mod mcp;
mod proxy;
mod refs;
mod route;
mod rewrite;
mod session;
mod shell;
mod squeeze;
mod terse;
#[cfg(test)] mod t_retention;
mod tomlfilters;
mod usage;
mod cmdviews;
mod views_build;
mod views_lang;
mod views_sys;
mod userfilters;
mod args;
mod cmd_chunks;
mod cmd_codec;
mod help;
mod jsonmap;

use help::usage;

/// Run the CLI with `args` (without the program name); returns the exit code.
pub fn run(args: &[String]) -> i32 {
    let Some(cmd) = args.first() else { usage() };
    let rest = &args[1..];
    let code = match cmd.as_str() {
        "mcp" => mcp::serve(rest),
        "languages" => cmd_codec::languages_cmd(),
        "grammar" => cmd_codec::grammar_cmd(rest),
        "encode" => cmd_codec::encode_cmd(rest),
        "decode" => cmd_codec::decode_cmd(rest),
        "roundtrip" | "stats" => cmd_codec::roundtrip_cmd(cmd, rest),
        "config" => config::run(rest),
        "run" => shell::run(rest),
        "gain" => usage::gain(rest),
        "session" => session::run(rest),
        "statusline" => session::statusline(rest),
        "hook" => hook::run(rest),
        "rewrite" => rewrite::run(rest),
        "browse" => browse::run(rest),
        "filter" => filter_cmd::run(rest),
        "route" => route::run(rest),
        "proxy" => proxy::run(rest),
        "auto" => auto::run(rest),
        "shims" => auto::shims_cmd(rest),
        "squeeze" => squeeze::run(rest),
        "discover" => discover::run(rest),
        "graph" => graph::run(rest),
        "init" => init::run(rest),
        "refs" => refs::run(rest),
        "map" => refs::run_map(rest),
        "find" => refs::run_find(rest),
        "output" if rest.iter().any(|a| a == "--file") => gitdiff::output_file(rest),
        "output" => shell::output_cmd(rest),
        "outline" => cmd_chunks::outline_cmd(rest),
        "symbols" => cmd_chunks::symbols_cmd(rest),
        "split" | "chunk" | "apply" => cmd_chunks::split_cmd(cmd, rest),
        _ => usage(),
    };
    code
}
