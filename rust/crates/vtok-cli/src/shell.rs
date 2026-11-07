//! `vtok run -- CMD` and `vtok output [ID]`: run a command, keep its full output on disk, show a compact view.
//!
//! No model involved. A per-tool view (views_*, filters) is used when one fits the command; otherwise the generic
//! summariser keeps every error/warning/summary line with a little context and collapses runs of near-identical lines.
//! Nothing is lost: the whole (ANSI-stripped) output is stored and `vtok output [ID]` pulls back any part of it; with no
//! id it reads the most recent run. A closing `# more: vtok output ID` line appears only when the view cut content or
//! the command failed (`# exit=N; ...`); a fully summarised success prints just the summary. If the view would not be
//! smaller than the output, or the output is short, the output is printed unchanged.
//!
//! One file per responsibility: clean (ANSI, line caps), score (what matters), summary (generic summariser), listing
//! (find/grep -rn), clean_run (green runs), failure (test failures by shape), table (padding), omitted (does a view admit
//! a cut), compact (the pipeline and the closing line), store (disk), exec (running), run (CLI/MCP), output (reading back).
mod clean;
mod clean_run;
mod compact;
mod exec;
mod failure;
mod listing;
mod omitted;
mod output;
mod run;
mod score;
mod store;
mod summary;
mod table;
#[cfg(test)] mod t_compact;
#[cfg(test)] mod t_declared;
#[cfg(test)] mod t_core;
#[cfg(test)] mod t_failure;
#[cfg(test)] mod t_store;

pub use clean::clean;
pub use compact::{compact, preview};
pub use output::{fetch, output_cmd, parse_range, Query};
pub use run::{run, run_for_mcp};
pub use store::{latest, load, state_dir};
pub(crate) use store::store;
