//! What a kind of call means for the numbers. `raw` is what the same thing would have cost without vtok, and how far
//! that comparison can be trusted depends on the tool: only `run` knows the real alternative (the command's own output).
//! Reading a file whole is an upper bound (an agent may have used grep or a ranged read), and the project-wide tools
//! compare against every file or a whole page, which nobody would have read, so they are listed but never counted.

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Class { Exact, Upper, NotCounted }

/// Most a shell tool shows of one command (about 30,000 characters): output beyond it was never going to be read.
pub const RUN_CAP: u64 = 8_000;
/// Most a file-read tool returns in one call.
pub const READ_CAP: u64 = 25_000;

pub fn class(kind: &str) -> Class {
    match kind { "run" => Class::Exact, "read" | "chunk" | "outline" => Class::Upper, _ => Class::NotCounted }
}

/// Tokens as the agent would have received them: capped at what its own tool delivers in one call.
pub fn counted(kind: &str, n: u64) -> u64 {
    match class(kind) { Class::Exact => n.min(RUN_CAP), Class::Upper => n.min(READ_CAP), Class::NotCounted => n }
}

/// Raw minus shown, both as counted; 0 for kinds whose raw is not a real alternative. Negative when vtok cost more.
pub fn saved(kind: &str, raw: u64, shown: u64) -> i64 {
    if class(kind) == Class::NotCounted { return 0; }
    counted(kind, raw) as i64 - counted(kind, shown) as i64
}

pub fn class_name(c: Class) -> &'static str {
    match c { Class::Exact => "exact", Class::Upper => "upper bound", Class::NotCounted => "not counted" }
}

/// Why a call that saved nothing saved nothing, read from its detail: `vtok run` prints a short output as it is ("tiny",
/// no loss, just no gain) and an output its view could not shrink ("not smaller").
pub fn passthrough(detail: &str) -> Option<&'static str> {
    if detail.contains("(not smaller") { Some("not smaller") } else if detail.contains("(tiny") { Some("tiny") } else { None }
}

/// What `raw` stands for, for the tables.
pub fn basis(kind: &str) -> &'static str {
    match kind {
        "run" => "exact: the command's own output (counted up to 8,000)",
        "read" => "upper bound: the whole file (counted up to 25,000)",
        "chunk" | "outline" => "upper bound: the whole file; a ranged read would be less",
        "output" => "not counted: reading back stored output costs extra",
        "route" => "not counted: no alternative to compare",
        _ => "not counted: the alternative is a whole project or page",
    }
}
