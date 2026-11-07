//! A whole file: front matter, fenced code, tables, indented code, HTML and rules are kept or dropped as a unit; the
//! prose lines in between go through `prose_line`.
use super::prose::prose_line;

pub fn squeeze(text: &str) -> String {
    let mut out: Vec<String> = vec![];
    let mut fence: Option<String> = None;
    let mut blank = 0;
    let mut front = false;
    for (n, raw) in text.lines().enumerate() {
        let t = raw.trim_start();
        if n == 0 && raw.trim() == "---" { front = true; out.push(raw.to_string()); continue; }
        if front { if raw.trim() == "---" { front = false; } out.push(raw.to_string()); continue; }
        if let Some(f) = &fence {
            out.push(raw.trim_end().to_string());
            if t.starts_with(f.as_str()) { fence = None; }
            continue;
        }
        if t.starts_with("```") || t.starts_with("~~~") { fence = Some(t[..3].to_string()); blank = 0; out.push(raw.trim_end().to_string()); continue; }
        if raw.trim().is_empty() { blank += 1; if blank == 1 { out.push(String::new()); } continue; }
        let indented_code = raw.starts_with("    ") && !t.starts_with(['-', '*', '+']) && !t.chars().next().map_or(false, |c| c.is_ascii_digit());
        let is_rule = t.len() >= 3 && t.chars().all(|c| matches!(c, '-' | '*' | '_' | ' ')) && !t.starts_with("| ");
        if is_rule { continue; }
        blank = 0;
        if indented_code || t.starts_with('|') || t.starts_with('<') { out.push(raw.trim_end().to_string()); continue; }
        out.push(prose_line(raw));
    }
    while out.last().map_or(false, |l| l.is_empty()) { out.pop(); }
    out.join("\n") + "\n"
}
