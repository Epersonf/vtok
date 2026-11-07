//! Money in one unit: what it costs to re-read one cached token (cache read 1, cache write 12.5, uncached input 10,
//! output 50: the list-price ratios of the Claude models). A token that rides in the prompt for L turns costs one write
//! and L-1 reads, so a token a tool did not put there saves that much, and a tool that adds tokens to every turn (the
//! MCP server's instructions) costs a read per turn.
use super::transcript::Usage;

pub const WRITE: f64 = 12.5;
pub const READ: f64 = 1.0;
pub const INPUT: f64 = 10.0;
pub const OUTPUT: f64 = 50.0;
/// Prompt tokens that registering the MCP server adds to every turn (measured with one-turn `claude -p` runs, see
/// bench/loop/RESULTS.md).
pub const STANDING_TOKENS: f64 = 729.0;
/// US$ per million uncached input tokens: the Sonnet-class list price, overridable with `--price-in`.
pub const DEFAULT_PRICE_IN: f64 = 3.0;

pub fn units(u: &Usage) -> f64 { u.cache_read as f64 * READ + u.cache_write as f64 * WRITE + u.input as f64 * INPUT + u.output as f64 * OUTPUT }

/// Units in US$: a cache read costs a tenth of the input price.
pub fn usd(units: f64, price_in: f64) -> f64 { units * price_in / 10.0 / 1e6 }

/// The cost of keeping a token in the prompt for `turns` turns: written once, then read.
pub fn ride(turns: usize) -> f64 { if turns == 0 { 0.0 } else { WRITE + READ * (turns - 1) as f64 } }

/// What the MCP server's instructions cost over `turns` turns; the first of them pays the write when it is in the window.
pub fn standing(turns: usize, includes_first_turn: bool) -> f64 {
    if turns == 0 { return 0.0; }
    STANDING_TOKENS * (READ * turns as f64 + if includes_first_turn { WRITE - READ } else { 0.0 })
}
