//! docker / podman / docker compose: ps, images, logs, build.
use super::util::*;

pub fn view(args: &[String], lines: &[String]) -> Option<String> {
    let pos = positional(args);
    match *pos.first()? {
        "ps" => ps(lines),
        "images" | "image" if pos.get(1).map_or(true, |s| *s == "ls") => images(lines),
        "logs" => super::envlog::log(lines).or_else(|| tail(lines, 30)),
        "build" | "buildx" => build(lines),
        "compose" => compose(&args[1..], lines),
        _ => None,
    }
}

pub fn compose(args: &[String], lines: &[String]) -> Option<String> {
    match positional(args).first().copied()? {
        "ps" => ps(lines),
        "logs" => super::envlog::log(lines).or_else(|| tail(lines, 30)),
        "build" => build(lines),
        "up" | "down" | "pull" | "stop" | "start" | "restart" => {
            let keep: Vec<String> = lines.iter().map(|l| l.trim().to_string())
                .filter(|l| { let u = l.to_lowercase(); u.contains("error") || u.contains("fail") || l.contains("Started") || l.contains("Healthy") || l.contains("Removed") || l.contains("Stopped") || l.contains("Created") }).collect();
            if keep.is_empty() { None } else { Some(keep.join("\n") + "\n") }
        }
        _ => None,
    }
}

fn tail(lines: &[String], n: usize) -> Option<String> {
    if lines.len() <= n { return None; }
    Some(format!("... {} earlier lines\n{}\n", lines.len() - n, lines[lines.len() - n..].join("\n")))
}

fn ps(lines: &[String]) -> Option<String> {
    let (h, rows) = table(lines)?;
    let col = |n: &str| h.iter().position(|x| x == n);
    let (name, image, status, ports) = (col("NAMES").or(col("NAME"))?, col("IMAGE")?, col("STATUS")?, col("PORTS"));
    let mut o = format!("{} containers\n", rows.len());
    for r in &rows {
        let p = ports.map(|i| r[i].split(", ").map(|s| s.replace("0.0.0.0:", "").replace("[::]:", "")).filter(|s| !s.starts_with(":::")).collect::<Vec<_>>()).unwrap_or_default();
        let p = { let mut u: Vec<String> = vec![]; for x in p { if !u.contains(&x) { u.push(x); } } u.join(",") };
        o.push_str(&format!("{} {} [{}] {}\n", r[name], r[image], r[status], p).replace(" \n", "\n"));
    }
    Some(o)
}

fn images(lines: &[String]) -> Option<String> {
    let (h, rows) = table(lines)?;
    let col = |n: &str| h.iter().position(|x| x == n);
    let (repo, tag, size) = (col("REPOSITORY")?, col("TAG")?, col("SIZE")?);
    let mut o = format!("{} images\n", rows.len());
    for r in rows.iter().take(40) { o.push_str(&format!("{}:{} {}\n", r[repo], r[tag], r[size])); }
    Some(o)
}

fn build(lines: &[String]) -> Option<String> {
    let mut o = String::new();
    let steps = lines.iter().filter(|l| l.starts_with('#') && l.contains(" DONE")).count();
    for l in lines {
        let u = l.to_lowercase();
        let important = u.contains("error") || u.contains("failed") || l.contains("naming to") || l.starts_with("Successfully") || l.contains("exporting to image");
        if important { o.push_str(&format!("{}\n", trunc(l.trim(), 200))); }
    }
    if o.is_empty() && steps == 0 { return None; }
    Some(format!("{steps} steps done\n{o}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn l(s: &str) -> Vec<String> { s.lines().map(String::from).collect() }
    fn a(s: &str) -> Vec<String> { s.split_whitespace().map(String::from).collect() }
    #[test]
    fn ps_compacts() {
        let raw = "CONTAINER ID   IMAGE          COMMAND                  CREATED        STATUS         PORTS                                       NAMES\nabc123def456   nginx:latest   \"/docker-entrypoint.…\"   2 hours ago    Up 2 hours     0.0.0.0:8080->80/tcp, :::8080->80/tcp       web\n789abc123def   postgres:16    \"docker-entrypoint.s…\"   3 days ago     Up 3 days      5432/tcp                                    db";
        let o = view(&a("ps"), &l(raw)).unwrap();
        assert!(o.contains("web nginx:latest [Up 2 hours] 8080->80/tcp") && o.contains("db postgres:16 [Up 3 days] 5432/tcp"));
        assert!(!o.contains("abc123def456") && !o.contains("entrypoint") && o.len() < raw.len());
    }
    #[test]
    fn images_and_build() {
        let raw = "REPOSITORY   TAG       IMAGE ID       CREATED        SIZE\nnginx        latest    a1b2c3d4e5f6   2 weeks ago    187MB\napp          dev       f6e5d4c3b2a1   1 hour ago     1.2GB";
        let o = view(&a("images"), &l(raw)).unwrap();
        assert!(o.contains("nginx:latest 187MB") && !o.contains("a1b2c3"));
        let b = l("#1 [internal] load build definition\n#1 DONE 0.1s\n#2 [1/3] FROM node:20\n#2 DONE 1.0s\n#3 RUN npm ci\n#3 0.5 npm warn\n#3 DONE 9.0s\n#4 exporting to image\n#4 naming to docker.io/library/app:dev done\n#4 DONE 0.1s");
        let o = view(&a("build ."), &b).unwrap();
        assert!(o.contains("4 steps done") && o.contains("naming to") && !o.contains("npm warn"));
    }
    #[test]
    fn logs_are_summarised() {
        let mut raw = String::new();
        for i in 0..30 { raw.push_str(&format!("2024-03-01T10:00:{:02}Z INFO tick {i}\n", i)); }
        raw.push_str("2024-03-01T10:01:00Z ERROR boom\n");
        let o = view(&a("logs web"), &l(&raw)).unwrap();
        assert!(o.contains("INFO 30") && o.contains("ERROR 1"));
    }
}
