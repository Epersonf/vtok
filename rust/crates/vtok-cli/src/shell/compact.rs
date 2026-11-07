//! From captured output to what the agent reads. A per-tool view when one fits, else the generic summariser; the whole
//! output is always stored. The id line is printed only when it is useful: the view left something out, or the command failed.
use super::clean::clean;
use super::clean_run::{is_pass_line, summarize_clean, without_noise, without_passes, CLEAN_BUDGET};
use super::failure::failure_view;
use super::listing::summarize_listing;
use super::omitted::admits_omission;
use super::score::score;
use super::store;
use super::summary::summarize;
use super::table::squeeze_table;
use vtok_api::ntok;

/// Output at most this long is shown as is.
const RAW_BELOW_TOKENS: usize = 250;
/// A clean run (exit 0, a summary line) is worth a dedicated view from this size on; the generic summariser from the next.
const CLEAN_MIN_TOKENS: usize = 24;
const CLEAN_GENERIC_TOKENS: usize = 60;
/// Id shown by previews (`vtok filter`, `discover`), which store nothing.
const PREVIEW_ID: &str = "00000000";

/// How much of the output the text carries.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Kind {
    /// The output itself (short, or the view was not smaller).
    Verbatim,
    /// A view that leaves out only chatter: counts, locations, messages are all there.
    Summary,
    /// A view that cut content (head/tail, "+N more", grouped listing): the rest is in the stored output.
    Partial,
}

pub struct Compacted { pub text: String, pub kind: Kind, pub id: Option<String> }

impl Compacted {
    /// The closing line, if one is needed: nothing for a verbatim or fully summarised success; the exit code for a
    /// failure; where the rest is stored when the view cut something. `with_exit` is false where the caller reports
    /// the exit status itself (the MCP tool).
    pub fn footer(&self, code: i32, with_exit: bool) -> Option<String> {
        let id = self.id.as_deref()?;
        match (self.kind, code != 0 && with_exit) {
            (Kind::Verbatim, _) => None,
            (Kind::Summary, false) => None,
            (Kind::Summary, true) | (Kind::Partial, true) => Some(format!("# exit={code}; more: vtok output {id}\n")),
            (Kind::Partial, false) => Some(format!("# more: vtok output {id}\n")),
        }
    }

    pub fn render(&self, code: i32) -> String { format!("{}{}", self.text, self.footer(code, true).unwrap_or_default()) }
}

fn with_newline(text: String) -> String { if text.is_empty() { text } else { text + "\n" } }

pub fn compact(raw: &str, code: i32, budget: usize) -> String { preview("", raw, code, budget) }

/// What `vtok run` prints for a finished command; the full output is stored.
pub fn compact_cmd(cmd: &str, raw: &str, code: i32, budget: usize) -> String { build(cmd, raw, code, budget, true).render(code) }

/// The same without storing anything (testing filters, estimating savings).
pub fn preview(cmd: &str, raw: &str, code: i32, budget: usize) -> String { build(cmd, raw, code, budget, false).render(code) }

pub fn build(cmd: &str, raw: &str, code: i32, budget: usize, persist: bool) -> Compacted {
    let orig_lines = clean(raw);
    let orig = orig_lines.join("\n");
    // Declarative filters rewrite the text first. Table squeezing is for text no per-tool view parses: those views
    // read the column padding (eslint's `message  rule`), so they see the unsqueezed lines.
    let filtered = crate::userfilters::apply(cmd, &orig_lines).or_else(|| crate::tomlfilters::apply(cmd, &orig_lines));
    let base = filtered.clone().unwrap_or_else(|| orig_lines.clone());
    let lines = filtered.or_else(|| squeeze_table(&orig_lines)).unwrap_or(orig_lines);
    let text = with_newline(lines.join("\n"));
    let id = if persist { store::store(&(orig.clone() + "\n")) } else { Some(PREVIEW_ID.to_string()) };
    let raw_tok = ntok(&orig);
    let verbatim = |text: String, id: Option<String>| Compacted { text, kind: Kind::Verbatim, id };
    // a clean run (exit 0, no error lines) needs only its summary, not a full head/tail
    let clean_run = code == 0 && !lines.iter().any(|l| score(l) >= 3 && !is_pass_line(l));
    let runner = lines.iter().any(|l| score(l) == 2);
    let small = if clean_run && runner { CLEAN_MIN_TOKENS } else { RAW_BELOW_TOKENS };
    if raw_tok <= small { return verbatim(text, id); }
    let tiny = clean_run && runner && raw_tok <= CLEAN_GENERIC_TOKENS;
    let Some((body, kind)) = pick(cmd, &base, &lines, budget, clean_run && runner, tiny) else { return verbatim(text, id) };
    let Some(id) = id else { return verbatim(text, None) };
    let shown = Compacted { text: body, kind, id: Some(id.clone()) };
    let shown_tok = ntok(&shown.render(code));
    if shown_tok >= raw_tok || shown.text.trim().is_empty() {
        if persist { crate::usage::record("run", raw_tok, raw_tok, "(not smaller: raw)"); }
        return verbatim(text, Some(id));
    }
    if persist { crate::usage::record("run", raw_tok, shown_tok, &id); }
    shown
}

fn summary_or_partial(v: String) -> (String, Kind) { let k = if admits_omission(&v) { Kind::Partial } else { Kind::Summary }; (v, k) }

/// The body for this output and how complete it is; None for "show the output as is".
fn pick(cmd: &str, base: &[String], lines: &[String], budget: usize, quiet: bool, tiny: bool) -> Option<(String, Kind)> {
    if let Some(v) = crate::cmdviews::view(cmd, base) { return Some(summary_or_partial(v)); }
    if tiny { return None; }
    if let Some(v) = crate::gitfmt::diff_view(base, budget).or_else(|| crate::gitfmt::log_view(base, budget)) { return Some((v, Kind::Partial)); }
    if let Some(v) = crate::filters::view(base) { return Some(summary_or_partial(v)); }
    let test_report = base.iter().any(|l| l.starts_with('=') && l.contains(" FAILURES "));
    let failure = failure_view(base).filter(|v| ntok(v) <= budget);
    if let Some(v) = failure.clone().filter(|_| test_report) { return Some((v, Kind::Partial)); }
    if let Some(l) = summarize_listing(lines, budget.min(150)) { return Some((l, Kind::Partial)); }
    if quiet { return Some((summarize_clean(&without_noise(lines), budget.min(CLEAN_BUDGET)), Kind::Partial)); }
    if let Some(v) = failure { return Some((v, Kind::Partial)); }
    Some((summarize(&without_noise(&without_passes(lines)), budget), Kind::Partial))
}
