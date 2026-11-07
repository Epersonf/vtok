//! The numeric settings of `vtok config`: the name on the command line, the key in the JSON files, the environment variable,
//! the default and the allowed range. A value is looked up in this order: environment, the nearest `.vtok.json`, the user
//! file, the default. A value outside its range is ignored, so a typo falls through to the next source.
use super::lookup::{local_config_path, read_json, user_config_path};
use std::path::Path;

pub struct Setting {
    pub name: &'static str,
    pub key: &'static str,
    pub env: &'static str,
    pub default: usize,
    pub min: usize,
    pub max: usize,
    pub about: &'static str,
}

pub static MAX_LINES: Setting = Setting {
    name: "max-lines", key: "max_lines", env: "VTOK_MAX_LINES", default: 150, min: 1, max: 100_000,
    about: "files longer than this are split into chunks",
};

pub static MIN_GAIN: Setting = Setting {
    name: "min-gain", key: "min_gain", env: "VTOK_MIN_GAIN", default: 10, min: 0, max: 90,
    about: "percent of tokens the compact form must save before vtok_read / vtok_chunk return it instead of the plain text",
};

pub static RUN_TINY: Setting = Setting {
    name: "run-tiny", key: "run_tiny", env: "VTOK_RUN_TINY", default: 250, min: 0, max: 20_000,
    about: "`vtok run` prints an output of up to this many tokens unchanged",
};

pub static ALL: [&Setting; 3] = [&MAX_LINES, &MIN_GAIN, &RUN_TINY];

/// A setting by its command-line name (`min-gain`) or its JSON key (`min_gain`).
pub fn find(name: &str) -> Option<&'static Setting> {
    ALL.iter().copied().find(|s| s.name == name || s.key == name)
}

impl Setting {
    /// The number, if it is a whole number inside the allowed range.
    pub fn parse(&self, text: &str) -> Option<usize> {
        text.trim().parse::<usize>().ok().filter(|n| (self.min..=self.max).contains(n))
    }

    pub fn in_file(&self, path: &Path) -> Option<usize> {
        read_json(path).get(self.key).and_then(|v| v.as_u64()).and_then(|n| self.parse(&n.to_string()))
    }

    /// Effective value and where it came from. `near` is the file being worked on (for the `.vtok.json` lookup).
    pub fn get(&self, near: Option<&Path>) -> (usize, String) {
        self.get_with(std::env::var(self.env).ok().as_deref(), near)
    }

    pub(super) fn get_with(&self, env: Option<&str>, near: Option<&Path>) -> (usize, String) {
        if let Some(n) = env.and_then(|s| self.parse(s)) { return (n, self.env.into()); }
        let start = near.map(Path::to_path_buf).or_else(|| std::env::current_dir().ok());
        if let Some(p) = start.as_deref().and_then(local_config_path) {
            if let Some(n) = self.in_file(&p) { return (n, p.display().to_string()); }
        }
        if let Some(p) = user_config_path() {
            if let Some(n) = self.in_file(&p) { return (n, p.display().to_string()); }
        }
        (self.default, "default".into())
    }
}
