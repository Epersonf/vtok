//! Which kind of command a shell call was: the program plus its subcommand ("cargo test", "git log", "python -m pytest").
//! A leading `cd DIR` says nothing about what ran, so it is skipped; every `for`/`while` loop is one kind.

/// The group a command belongs to in the report.
pub fn key(cmd: &str) -> String {
    let cmd = crate::rewrite::split_prefix(after_cd(cmd)).1;
    let w: Vec<&str> = cmd.split_whitespace().skip_while(|w| w.contains('=') && !w.starts_with('-') && !w.contains('/')).collect();
    let prog = w.first().map(|p| p.rsplit('/').next().unwrap_or(p)).unwrap_or("?");
    match (prog, w.get(1)) {
        ("python" | "python3", Some(&"-m")) => format!("python -m {}", w.get(2).unwrap_or(&"")),
        ("for" | "while" | "until", _) => "shell loop".to_string(),
        (_, Some(s)) if !s.starts_with('-') && !s.contains('/') && !s.contains('.') => format!("{prog} {s}"),
        _ => prog.to_string(),
    }
}

/// The command after any leading `cd DIR` or `export VAR=x` that ends in `&&`, `;` or a newline.
fn after_cd(mut cmd: &str) -> &str {
    loop {
        let t = cmd.trim_start();
        match t.strip_prefix("cd ").or_else(||t.strip_prefix("export ")).and_then(after_separator) {
            Some(rest) => cmd = rest,
            None => return t,
        }
    }
}

/// What follows the first `&&`, `;` or newline that is outside quotes.
fn after_separator(t: &str) -> Option<&str> {
    let b = t.as_bytes();
    let mut quote = 0u8;
    for (i, &c) in b.iter().enumerate() {
        if quote != 0 {
            if c == quote { quote = 0 }
        } else if c == b'"' || c == b'\'' {
            quote = c;
        } else if c == b';' || c == b'\n' {
            return Some(&t[i + 1..]);
        } else if c == b'&' && b.get(i + 1) == Some(&b'&') {
            return Some(&t[i + 2..]);
        }
    }
    None
}
