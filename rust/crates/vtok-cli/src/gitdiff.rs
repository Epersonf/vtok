//! The view for patches: `git diff` (any form: --cached, A..B, -U0, --word-diff, colour), `git show`, `git log -p`,
//! `git stash show -p`, combined merge diffs and `gh pr diff`; `--stat` / `--name-status` pass through.
//!
//! Up to ~400 tokens a patch is shown as is. Above that: one header line per file, always all of them
//! (`M path (+a -d)`, `A`, `D`, `R old -> new (98%)`, binary, mode, generated/lock, whitespace-only), then the changed
//! lines of each hunk under `@@ ... @@ symbol`, without `diff --git`/`index`/`---`/`+++`, context trimmed to one line
//! (zero when the budget is tight), long lines cut, repeated hunks collapsed. The budget scales with the patch
//! (see `budget_for`) and `plan` decides who gets it; whatever is cut is said, and `vtok output ID --file PATH`
//! prints one file's full section from the stored output.
//!
//! Files: model (types, classifiers), parse (text -> items), rows (display lines + costs), plan (budget),
//! render (headers, commit text, assembly), listing (--stat / --name-status), section (`--file`).
mod listing;
mod model;
mod parse;
mod plan;
mod render;
mod rows;
mod section;
#[cfg(test)]
mod t_kinds;
#[cfg(test)]
mod t_budget;
#[cfg(test)]
mod t_variants;

use std::collections::HashMap;
use vtok_api::ntok;
pub use parse::is_file_start;
pub use section::output_file;

/// Raw patches up to this size are printed unchanged.
pub const SMALL: usize = 400;

/// Token budget for a patch of `raw` tokens: 2500 + raw/2, at most 8000 (so every changed line of a patch up to ~5000
/// tokens fits once boilerplate and context are gone). Tuned with benchmarks/diff_eval.py to keep at least RTK's
/// changed-line retention while printing fewer tokens. `asked` (the caller's --max-tokens) only raises it.
pub fn budget_for(raw: usize, asked: usize) -> usize {
    (2500 + raw / 2).min(8000).max(asked)
}

/// One line of context around changes only when that still costs at most this share of the raw patch, uncut.
const CTX_SHARE: usize = 60;

pub fn diff_view(lines: &[String], budget: usize) -> Option<String> {
    if !lines.iter().any(|l| parse::is_file_start(l)) { return listing::view(lines, budget); }
    let raw = ntok(&lines.join("\n"));
    if raw <= SMALL { return Some(lines.join("\n") + "\n"); }
    let items = parse::parse(lines);
    let files: Vec<&model::FileDiff> = items.iter().filter_map(|i| if let model::Item::File(f) = i { Some(f) } else { None }).collect();
    let (a, d) = files.iter().map(|f| f.counts()).fold((0, 0), |(x, y), (a, d)| (x + a, y + d));
    let top = format!("# {} files +{a} -{d}\n", files.len());
    let foot = "# cut parts: vtok output ID --file PATH prints that file's whole diff\n";
    let b = budget_for(raw, budget);
    let base = ntok(&top) + ntok(foot) + render::text_cost(&items);
    for ctx in [1, 0] {
        let mut seen = HashMap::new();
        let mut es: Vec<plan::Entry> = files.iter().map(|f| render::entry(f, ctx, &mut seen)).collect();
        let b = if ctx == 1 { b.min(raw * CTX_SHARE / 100) } else { b };
        plan::allocate(&mut es, b.saturating_sub(base));
        let (body, cut) = render::render(&items, &es);
        if cut && ctx == 1 { continue; }        return Some(format!("{top}{body}{}", if cut { foot } else { "" }));
    }
    None
}
