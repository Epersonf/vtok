//! One line of the usage log. Records written since the log carries tags have `"v": 2`; older ones lack it (and may
//! include test and benchmark runs, so reports leave them out unless asked).
use super::context::{cwd, session_id, via};
use serde_json::{json, Value};

pub const VERSION: u64 = 2;
const DETAIL_CHARS: usize = 80;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Rec {
    pub t: u64,
    pub kind: String,
    /// Tokens of the alternative (the whole file, the command's full output); how far to trust it depends on the kind.
    pub raw: u64,
    /// Tokens vtok returned.
    pub shown: u64,
    pub detail: String,
    /// The agent's chat (Claude Code's session id) or `VTOK_SESSION`; empty when unknown.
    pub session: String,
    pub cwd: String,
    /// "mcp" for a tool call served by `vtok mcp`, "cli" for the command line (and the hook's `vtok run`).
    pub via: String,
    /// False for records written before the tags existed.
    pub tagged: bool,
}

impl Rec {
    /// A record for a call made now by this process.
    pub fn now(kind: &str, raw: usize, shown: usize, detail: &str) -> Rec {
        let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
        Rec { t, kind: kind.into(), raw: raw as u64, shown: shown as u64, detail: detail.chars().take(DETAIL_CHARS).collect(),
              session: session_id().unwrap_or_default(), cwd: cwd(), via: via().into(), tagged: true }
    }

    pub fn to_json(&self) -> Value {
        let mut v = json!({"v": VERSION, "t": self.t, "kind": self.kind, "raw": self.raw, "shown": self.shown,
                           "detail": self.detail, "cwd": self.cwd, "via": self.via});
        if !self.session.is_empty() { v["s"] = json!(self.session); }
        v
    }

    pub fn from_json(v: &Value) -> Option<Rec> {
        let text = |k: &str| v[k].as_str().unwrap_or("").to_string();
        Some(Rec { t: v["t"].as_u64()?, kind: v["kind"].as_str()?.to_string(), raw: v["raw"].as_u64().unwrap_or(0),
                   shown: v["shown"].as_u64().unwrap_or(0), detail: text("detail"), session: text("s"), cwd: text("cwd"),
                   via: text("via"), tagged: v["v"].as_u64().unwrap_or(0) >= VERSION })
    }
}
