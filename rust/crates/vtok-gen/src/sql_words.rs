//! Lexical reading of one SQL statement (works on statements the grammar cannot parse): comments dropped,
//! whitespace collapsed, words with quoted names and strings kept whole.
use crate::lex::regions;
use crate::spec::Spec;

/// The statement without comments, whitespace collapsed outside strings; dollar-quoted bodies of more than one line
/// become `$$…N lines$$` when `bodies` is false.
pub fn flatten(spec: &Spec, s: &str, bodies: bool) -> String {
    let (mut parts, mut at): (Vec<String>, usize) = (vec![], 0);
    for (a, e, string) in regions(spec, s) {
        parts.extend(s[at..a].split_whitespace().map(String::from));
        let lit = &s[a..e];
        let lines = lit.lines().count();
        if string && !bodies && lit.starts_with('$') && lines > 1 { parts.push(format!("$$…{lines} lines$$")); } else if string { parts.push(lit.to_string()); }
        at = e;
    }
    parts.extend(s[at.min(s.len())..].split_whitespace().map(String::from));
    let mut out = String::new();
    for p in parts {
        if !out.is_empty() && !out.ends_with('(') && !p.starts_with([',', ')', ';']) { out.push(' '); }
        out.push_str(&p);
    }
    out
}

/// Words: punctuation `( ) , ;` alone, quoted names and strings whole.
pub fn words(s: &str) -> Vec<String> {
    let (mut out, mut cur, mut quote) = (vec![], String::new(), None::<char>);
    for c in s.chars() {
        if let Some(q) = quote { cur.push(c); if c == q { quote = None; } continue; }
        match c {
            '"' | '\'' | '`' => { cur.push(c); quote = Some(c); }
            '(' | ')' | ',' | ';' => { if !cur.is_empty() { out.push(std::mem::take(&mut cur)); } out.push(c.to_string()); }
            c if c.is_whitespace() => if !cur.is_empty() { out.push(std::mem::take(&mut cur)); },
            _ => cur.push(c),
        }
    }
    if !cur.is_empty() { out.push(cur); }
    out
}

const MODS: &[&str] = &["or", "replace", "unique", "temp", "temporary", "materialized", "recursive", "global", "local", "unlogged", "constraint", "trusted", "procedural", "external", "virtual"];

/// The outline name of a statement: `create table public.users`, `create index i on t`, `alter table t`,
/// `insert into t (3 rows)`, `create policy p on t`...
pub fn label(w: &[String]) -> String {
    let lw: Vec<String> = w.iter().map(|x| x.to_ascii_lowercase()).collect();
    let at = |i: usize| lw.get(i).map(String::as_str).unwrap_or("");
    let name_after = |mut i: usize| {
        while matches!(at(i), "if" | "not" | "exists" | "only" | "concurrently") { i += 1; }
        (w.get(i).cloned().unwrap_or_default(), i)
    };
    match at(0) {
        "create" => {
            let mut i = 1;
            while MODS.contains(&at(i)) { i += 1; }
            let obj = at(i).to_string();
            let (name, j) = name_after(i + 1);
            let on = (j + 1..lw.len().min(j + 40)).find(|&k| at(k) == "on").and_then(|k| w.get(k + 1)).filter(|_| matches!(obj.as_str(), "index" | "policy" | "trigger" | "rule"));
            let name = if name == "on" { String::new() } else { name };
            let s = format!("create {obj} {name}");
            match on { Some(t) => format!("{} on {t}", s.trim_end()), None => s.trim_end().to_string() }
        }
        "alter" | "drop" | "truncate" => { let (name, _) = name_after(2); format!("{} {} {name}", at(0), at(1)).trim_end().to_string() }
        "comment" => format!("comment on {} {}", at(2), w.get(3).cloned().unwrap_or_default()),
        "insert" => {
            let t = w.get(2).cloned().unwrap_or_default();
            match lw.iter().position(|x| x == "values") {
                Some(v) => format!("insert into {t} ({} rows)", rows(&w[v + 1..])),
                None => format!("insert into {t}"),
            }
        }
        "update" => format!("update {}", w.get(1).cloned().unwrap_or_default()),
        "delete" => format!("delete from {}", w.get(2).cloned().unwrap_or_default()),
        "select" | "with" => match lw.iter().position(|x| x == "from") { Some(f) => format!("{} from {}", at(0), w.get(f + 1).cloned().unwrap_or_default()), None => at(0).to_string() },
        _ => { let s = w.iter().take(8).cloned().collect::<Vec<_>>().join(" ").replace(" ,", ",").replace("( ", "(").replace(" )", ")"); if s.chars().count() > 60 { s.chars().take(60).collect::<String>() + "…" } else { s } }
    }
}

/// Top-level parenthesized groups in `w` (the rows after VALUES).
pub fn rows(w: &[String]) -> usize {
    let (mut depth, mut n) = (0i32, 0);
    for x in w {
        match x.as_str() { "(" => { if depth == 0 { n += 1; } depth += 1; } ")" => depth -= 1, ";" if depth == 0 => break, _ => {} }
    }
    n
}
