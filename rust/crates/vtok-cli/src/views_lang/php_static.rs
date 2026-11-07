//! phpstan / ecs table reporters and pint: `file:line message` per finding, tips and diffs dropped.
use super::util::*;

/// phpstan (`Line  file` table) and ecs (same table plus a diff and the applied checkers).
pub fn table(lines: &[String], ecs: bool) -> Option<String> {
    let file_head = re(r"^\s*Line\s+(\S.*?)\s*$");
    let entry = re(r"^\s{1,3}(\d+)\s{2,}(\S.*)$");
    let raw = re(r"^(\S+\.php):(\d+):(.*)$");
    let (mut file, mut out) = (String::new(), Vec::<String>::new());
    let (mut in_diff, mut in_checkers) = (false, false);
    for l in lines {
        let t = l.trim();
        if let Some(c) = file_head.captures(l) { file = c[1].to_string(); in_diff = false; in_checkers = false; }
        else if let Some(c) = entry.captures(l) {
            in_checkers = false;
            in_diff = ecs && c[2].contains("begin diff");
            let msg = if in_diff { "style violation".to_string() } else { short(&c[2], 200) };
            out.push(format!("{}:{} {}", file, &c[1], msg));
        } else if let Some(c) = raw.captures(l) { out.push(format!("{}:{} {}", &c[1], &c[2], short(c[3].trim(), 200))); }
        else if ecs && t.contains("begin diff") { in_diff = true; }
        else if ecs && t.contains("end diff") { in_diff = false; }
        else if ecs && t.starts_with("Applied checkers") { in_checkers = true; }
        else if ecs && in_checkers && t.starts_with("* ") {
            if let Some(last) = out.last_mut() { if !last.contains(" [") { last.push_str(&format!(" [{}", t[2..].rsplit('\\').next().unwrap_or(""))); } else { last.push_str(&format!(", {}", t[2..].rsplit('\\').next().unwrap_or(""))); } }
        } else if !in_diff && !in_checkers && !ecs && l.starts_with("         ") && !t.starts_with('💡') && !t.starts_with("Tip") && out.last().map_or(false, |p| p.len() < 120 && !p.ends_with(']')) {
            // wrapped message continuation (phpstan)
            if let Some(last) = out.last_mut() { last.push(' '); last.push_str(t); }
        }
    }
    let tail = grep(lines, r"^\s*\[(ERROR|OK|WARNING)\]|^\s*Found \d+ errors?");
    if out.is_empty() && tail.is_empty() { return None; }
    if ecs { for o in out.iter_mut() { if o.contains(" [") { o.push(']'); } } }
    let mut v = cap(out, 80);
    v.extend(tail);
    finish(v)
}

pub fn pint(lines: &[String]) -> Option<String> {
    let out = grep(lines, r"^\s*[⨯~] \S|^\s*(FIXED|FAIL|PASS|DONE)\b|style issues?");
    if out.is_empty() { return None; }
    finish(cap(collapse(out), 40))
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::util::t::*;

    #[test]
    fn phpstan_table() {
        let s = " ------ ----------------------------------------------\n  Line   src/Foo.php\n ------ ----------------------------------------------\n  12     Call to an undefined method App\\Foo::bar().\n         💡 Tip of the Day: use @method\n  30     Parameter #1 $id of method App\\Foo::find() expects int,\n         string given.\n ------ ----------------------------------------------\n\n ------ ----------------------------------------------\n  Line   src/Bar.php\n ------ ----------------------------------------------\n  4      Property App\\Bar::$x is never read.\n ------ ----------------------------------------------\n\n [ERROR] Found 3 errors\n";
        let l = lines(s);
        let o = small(table(&l, false), &l);
        assert!(o.contains("src/Foo.php:12 Call to an undefined method") && o.contains("src/Foo.php:30 Parameter #1") && o.contains("string given."));
        assert!(o.contains("src/Bar.php:4 Property") && o.contains("[ERROR] Found 3 errors"));
        assert!(!o.contains("----") && !o.contains("Tip of the Day"));
        assert!(table(&lines(" [OK] No errors"), false).unwrap().contains("[OK] No errors"));
    }

    #[test]
    fn ecs_drops_diff_keeps_checkers() {
        let s = " 5/5 [▓▓▓▓▓] 100%\n\n ------ ---------------------------------\n  Line   src/Foo.php\n ------ ---------------------------------\n  12     ---------- begin diff ----------\n         @@ -10,7 +10,7 @@\n         -    $a = array();\n         +    $a = [];\n         ----------- end diff -----------\n         Applied checkers:\n          * PhpCsFixer\\Fixer\\ArrayNotation\\ArraySyntaxFixer\n ------ ---------------------------------\n\n [ERROR] Found 1 error\n";
        let l = lines(s);
        let o = small(table(&l, true), &l);
        assert!(o.contains("src/Foo.php:12") && o.contains("ArraySyntaxFixer") && o.contains("Found 1 error"));
        assert!(!o.contains("$a = array") && !o.contains("5/5"));
    }

    #[test]
    fn pint_failures_only() {
        let s = "\n  ⨯⨯✓✓✓✓✓\n\n  ────────────────────────────── Laravel\n    FAIL   ................................ 7 files, 2 style issues\n  ⨯ app/Models/User.php        single_quote, ordered_imports\n  ⨯ app/Http/Kernel.php        no_unused_imports\n  ✓ app/Providers/App.php\n  ✓ app/Providers/Route.php\n";
        let l = lines(s);
        let o = small(pint(&l), &l);
        assert!(o.contains("app/Models/User.php") && o.contains("2 style issues") && !o.contains("Providers") && !o.contains("⨯⨯"));
    }
}
