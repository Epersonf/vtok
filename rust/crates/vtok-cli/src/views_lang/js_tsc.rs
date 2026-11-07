//! prettier --check and tsc: every offending file / every compiler error location, nothing else.
use super::util::*;

pub fn prettier(lines: &[String]) -> Option<String> {
    let ok = has(lines, r"All matched files use Prettier code style");
    let warn = re(r"^\[warn\] (.+)$");
    let mut files = vec![];
    let mut out = vec![];
    for l in lines {
        if let Some(c) = warn.captures(l) {
            let m = c[1].trim().to_string();
            if m.starts_with("Code style issues") { out.push(m); } else { files.push(m); }
        } else if l.starts_with("[error]") { out.push(l.trim().to_string()); }
    }
    if ok { return finish(vec!["prettier: all files formatted".into()]); }
    if files.is_empty() && out.is_empty() { return None; }
    let n = files.len();
    let mut v = vec![format!("prettier: {n} file(s) need formatting")];
    v.extend(cap(files, 30));
    v.extend(out);
    finish(v)
}

pub fn tsc(lines: &[String]) -> Option<String> {
    let plain = re(r"^(.+?)\((\d+),(\d+)\): (error|warning) (TS\d+): (.*)$");
    let pretty = re(r"^(.+?):(\d+):(\d+) - (error|warning) (TS\d+): (.*)$");
    let (mut errs, mut codes) = (vec![], vec![]);
    for l in lines {
        if let Some(c) = plain.captures(l).or_else(|| pretty.captures(l)) {
            codes.push(c[5].to_string());
            errs.push(format!("{}:{}:{} {} {}", &c[1], &c[2], &c[3], &c[5], short(&c[6], 220)));
        } else if !errs.is_empty() && re(r"^\s{2,}\S").is_match(l) && !re(r"^\s*(\d+\s|~)").is_match(l) && !l.trim().starts_with('~') {
            let last = errs.last_mut().unwrap();
            if last.matches(" | ").count() < 2 { last.push_str(&format!(" | {}", short(l.trim(), 120))); }
        }
    }
    let found = grep(lines, r"^Found \d+ errors?");
    if errs.is_empty() && found.is_empty() { return None; }
    let mut out = cap(errs, 60);
    if codes.len() > 5 { out.push(format!("by code: {}", tally(&codes))); }
    out.extend(found);
    finish(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::util::t::*;

    #[test]
    fn prettier_lists_files() {
        let l = lines("Checking formatting...\n[warn] src/a.ts\n[warn] src/b.ts\n[warn] Code style issues found in 2 files. Run Prettier with --write to fix.\n");
        let o = prettier(&l).unwrap();
        assert!(o.contains("2 file(s)") && o.contains("src/b.ts") && o.contains("Code style issues found in 2 files"));
        assert!(!o.contains("Checking formatting"));
        let ok = prettier(&lines("Checking formatting...\nAll matched files use Prettier code style!")).unwrap();
        assert!(ok.contains("all files formatted"));
    }

    #[test]
    fn tsc_keeps_locations_drops_frames() {
        let s = "src/a.ts:10:5 - error TS2322: Type 'string' is not assignable to type 'number'.\n\n10   const x: number = 'a';\n           ~\n\nsrc/b.ts:3:1 - error TS2304: Cannot find name 'foo'.\n\n3 foo();\n  ~~~\n\nFound 2 errors in 2 files.\n\nErrors  Files\n     1  src/a.ts:10\n     1  src/b.ts:3\n";
        let l = lines(s);
        let o = small(tsc(&l), &l);
        assert!(o.contains("src/a.ts:10:5 TS2322") && o.contains("src/b.ts:3:1 TS2304") && o.contains("Found 2 errors in 2 files."));
        assert!(!o.contains("~") && !o.contains("const x"));
        let p = lines("src/c.ts(4,2): error TS2551: Property 'nme' does not exist.\nFound 1 error in src/c.ts:4");
        assert!(tsc(&p).unwrap().contains("src/c.ts:4:2 TS2551"));
        assert!(tsc(&lines("fine")).is_none());
    }
}
