//! The parsed shape of a git patch: a sequence of commit headers and per-file sections, plus the line and path
//! classifiers the view uses (changed vs context, generated/lock/vendored paths, whitespace-only changes).

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Status { Mod, Add, Del, Rename, Copy }

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Kind { Add, Del, Ctx, NoNl }

pub struct Hunk {
    pub head: String,
    pub body: Vec<String>,
    /// marker columns: 1 for a normal diff, N for a combined diff of N parents, 0 for `--word-diff`
    pub width: usize,
}

pub struct FileDiff {
    pub old: String,
    pub new: String,
    pub status: Status,
    pub sim: Option<u32>,
    pub binary: bool,
    pub mode: Option<String>,
    pub combined: bool,
    pub hunks: Vec<Hunk>,
}

pub enum Item { Text(Vec<String>), File(FileDiff) }

pub fn kind(line: &str, width: usize) -> Kind {
    if line.starts_with('\\') { return Kind::NoNl; }
    if width == 0 {
        let (a, d) = word_marks(line);
        return if a { Kind::Add } else if d { Kind::Del } else { Kind::Ctx };
    }
    let p = line.get(..width.min(line.len())).unwrap_or("");
    if p.contains('-') { Kind::Del } else if p.contains('+') { Kind::Add } else { Kind::Ctx }
}

/// `--word-diff` marks: `{+added+}` / `[-removed-]` inline, or porcelain `+word` / `-word` lines.
pub fn word_marks(line: &str) -> (bool, bool) {
    let a = (line.contains("{+") && line.contains("+}")) || line.starts_with('+');
    let d = (line.contains("[-") && line.contains("-]")) || line.starts_with('-');
    (a, d)
}

impl FileDiff {
    pub fn path(&self) -> &str { if self.status == Status::Del { &self.old } else { &self.new } }

    pub fn counts(&self) -> (usize, usize) {
        let (mut a, mut d) = (0, 0);
        for h in &self.hunks {
            for l in &h.body {
                if h.width == 0 { let (x, y) = word_marks(l); a += x as usize; d += y as usize; continue; }
                match kind(l, h.width) { Kind::Add => a += 1, Kind::Del => d += 1, _ => {} }
            }
        }
        (a, d)
    }

    /// Every change run (consecutive -/+ lines) removes exactly what it adds, ignoring all whitespace: re-indentation,
    /// trailing spaces, CRLF <-> LF (the cleaner already strips `\r`, so line-ending flips arrive as identical pairs).
    pub fn whitespace_only(&self) -> bool {
        let squash = |l: &str, w: usize| l.get(w..).unwrap_or("").chars().filter(|c| !c.is_whitespace()).collect::<String>();
        let mut any = false;
        for h in self.hunks.iter().filter(|h| h.width == 1) {
            let (mut rm, mut ad): (Vec<String>, Vec<String>) = (vec![], vec![]);
            let mut runs: Vec<(Vec<String>, Vec<String>)> = vec![];
            for l in &h.body {
                match kind(l, 1) {
                    Kind::Del => rm.push(squash(l, 1)),
                    Kind::Add => ad.push(squash(l, 1)),
                    Kind::NoNl => {}
                    Kind::Ctx => if !rm.is_empty() || !ad.is_empty() { runs.push((std::mem::take(&mut rm), std::mem::take(&mut ad))); }
                }
            }
            if !rm.is_empty() || !ad.is_empty() { runs.push((rm, ad)); }
            for (r, a) in runs { if r != a { return false; } any = true; }
        }
        any && self.hunks.iter().all(|h| h.width == 1)
    }

    /// Lock files, generated, vendored, minified and snapshot files: one line, never their content.
    pub fn generated(&self) -> bool {
        is_generated(self.path()) || self.hunks.iter().flat_map(|h| h.body.iter()).take(50).any(|l| l.len() > 2000)
    }
}

pub fn is_generated(path: &str) -> bool {
    let base = path.rsplit('/').next().unwrap_or(path);
    const LOCKS: [&str; 12] = ["Cargo.lock", "package-lock.json", "yarn.lock", "pnpm-lock.yaml", "go.sum", "poetry.lock", "uv.lock",
        "composer.lock", "Gemfile.lock", "Pipfile.lock", "npm-shrinkwrap.json", "bun.lockb"];
    const DIRS: [&str; 5] = ["dist/", "vendor/", "node_modules/", "__snapshots__/", "third_party/"];
    LOCKS.contains(&base) || base.contains(".min.") || base.ends_with(".snap") || base.ends_with(".pb.go") || base.ends_with("_pb2.py")
        || base.ends_with(".generated.ts") || base.ends_with(".g.dart")
        || DIRS.iter().any(|d| path.starts_with(d) || path.contains(&format!("/{d}")))
}
