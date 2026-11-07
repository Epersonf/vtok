//! Dedicated views for system, cloud and git tooling (see cmdviews.rs for the contract).
mod util;
mod git;
mod git_ops;
mod git_misc;
mod forge;
mod files;
mod find_wc;
mod envlog;
mod jsonv;
mod docker;
mod kube;
mod net;
mod psql;
mod misc;

const FILE_PRINTERS: [&str; 6] = ["cat", "type", "bat", "head", "tail", "less"];

/// True when `cmd` prints a named file (`cat`, `head`, `bat`...): what the file is was decided from its name already.
pub fn prints_file(cmd: &str) -> bool {
    util::parse(cmd).is_some_and(|(tool, args)| FILE_PRINTERS.contains(&tool.as_str()) && !util::positional(&args).is_empty())
}

pub fn view(cmd: &str, lines: &[String]) -> Option<String> {
    let (tool, args) = util::parse(cmd)?;
    let logfile = args.iter().any(|a| a.ends_with(".log"));
    let out = match tool.as_str() {
        "git" => git::view(&args, lines),
        "gh" | "glab" => forge::forge(&args, lines),
        "gt" => forge::graphite(lines),
        "ls" | "dir" => files::ls(lines),
        "tree" => files::tree(lines),
        "find" | "fd" => find_wc::find(lines),
        "wc" => find_wc::wc(lines),
        "env" | "printenv" if args.is_empty() => envlog::env(lines),
        "journalctl" | "dmesg" => envlog::log(lines),
        "tail" | "cat" | "type" | "less" if logfile => envlog::log(lines),
        "docker" | "podman" => docker::view(&args, lines),
        "docker-compose" => docker::compose(&args, lines),
        "kubectl" | "k" | "oc" => kube::view(&args, lines),
        "aws" => net::aws(&args, lines),
        "curl" => net::curl(lines),
        "wget" => net::wget(lines),
        "psql" => psql::view(lines),
        "ast-grep" | "sg" | "sgrep" => misc::astgrep(lines),
        "cat" | "type" | "bat" | "head" | "tail" | "less" => {
            let ext = util::positional(&args).last().and_then(|f| f.rsplit_once('.')).map(|x| x.1);
            let json = args.iter().any(|a| a.ends_with(".json"));
            // a long list of similar records (CSV, JSON, YAML) first; otherwise the structure or the source-file cut
            crate::data::view(lines, ext).or_else(|| if json { jsonv::json_text(lines) } else { misc::file_dump(&args, lines) })
        }
        _ => None,
    };
    out.filter(|o| !o.is_empty()).filter(|o| util::smaller(o, lines))
}

#[cfg(test)]
mod tests_dispatch {
    use super::*;
    #[test]
    fn unknown_tool_is_none() {
        assert!(view("make all", &["x".to_string()]).is_none());
    }
    #[test]
    fn dispatches_through_prefixes() {
        let raw = "On branch main\nYour branch is up to date with 'origin/main'.\n\nChanges not staged for commit:\n  (use \"git add <file>...\" to update what will be committed)\n  (use \"git restore <file>...\" to discard changes in working directory)\n\tmodified:   a.rs\n\nno changes added to commit (use \"git add\" and/or \"git commit -a\")";
        let lines: Vec<String> = raw.lines().map(String::from).collect();
        let o = view("cd repo && GIT_PAGER=cat git -C x status", &lines).unwrap();
        assert!(o.contains("changed (1): M a.rs") && o.len() < raw.len());
        assert!(view("git status | head", &lines).is_none());
    }
}
