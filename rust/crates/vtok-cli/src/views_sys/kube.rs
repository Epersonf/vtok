//! kubectl: get (tables), describe (key sections), logs.
use super::util::*;

pub fn view(args: &[String], lines: &[String]) -> Option<String> {
    match positional(args).first().copied()? {
        "get" if !args.iter().any(|a| a == "-o" || a.starts_with("-o") || a == "--output") || args.iter().any(|a| a == "wide" || a == "-owide") => get(lines),
        "describe" => describe(lines),
        "logs" => super::envlog::log(lines).or_else(|| {
            (lines.len() > 30).then(|| format!("... {} earlier lines\n{}\n", lines.len() - 30, lines[lines.len() - 30..].join("\n")))
        }),
        _ => None,
    }
}

const DROP: [&str; 7] = ["AGE", "IP", "NODE", "NOMINATED NODE", "READINESS GATES", "SELECTOR", "CONTAINERS"];
const HEALTHY: [&str; 9] = ["Running", "Completed", "Succeeded", "Active", "Ready", "Bound", "True", "Available", "Complete"];

fn get(lines: &[String]) -> Option<String> {
    // `get -A` / multiple kinds print an extra NAMESPACE column; the table parser handles it.
    let (h, rows) = table(lines)?;
    if !h.iter().any(|c| c == "NAME") { return None; }
    let keep: Vec<usize> = (0..h.len()).filter(|&i| !DROP.contains(&h[i].as_str())).collect();
    let status = h.iter().position(|c| c == "STATUS");
    let fmt = |r: &Vec<String>| keep.iter().map(|&i| r[i].clone()).collect::<Vec<_>>().join(" ");
    let mut o = format!("{} items ({})\n", rows.len(), keep.iter().map(|&i| h[i].to_lowercase()).collect::<Vec<_>>().join(" "));
    let bad: Vec<&Vec<String>> = rows.iter().filter(|r| status.map_or(false, |s| !HEALTHY.iter().any(|h| r[s].starts_with(h)))).collect();
    if rows.len() <= 25 { for r in &rows { o.push_str(&format!("{}\n", fmt(r))); } return Some(o); }
    if let Some(s) = status {
        let mut counts: Vec<(String, usize)> = vec![];
        for r in &rows { match counts.iter_mut().find(|(n, _)| *n == r[s]) { Some(c) => c.1 += 1, None => counts.push((r[s].clone(), 1)) } }
        o.push_str(&format!("status: {}\n", counts.iter().map(|(n, c)| format!("{n} {c}")).collect::<Vec<_>>().join(", ")));
    }
    for r in bad.iter().take(20) { o.push_str(&format!("{}\n", fmt(r))); }
    if bad.is_empty() { for r in rows.iter().take(10) { o.push_str(&format!("{}\n", fmt(r))); } o.push_str("... rest healthy\n"); }
    Some(o)
}

const SCALARS: [&str; 10] = ["Name:", "Namespace:", "Node:", "Status:", "IP:", "Type:", "Replicas:", "Reason:", "Image:", "Port:"];
const CONTAINER_KEYS: [&str; 7] = ["Image:", "State:", "Ready:", "Restart Count:", "Reason:", "Exit Code:", "Message:"];

fn describe(lines: &[String]) -> Option<String> {
    if !lines.iter().any(|l| l.starts_with("Name:")) { return None; }
    let mut o = String::new();
    let mut section = "";
    let mut events: Vec<&str> = vec![];
    for l in lines {
        if !l.starts_with(' ') && !l.is_empty() {
            section = l.split(':').next().unwrap_or("");
            if section == "Events" || section == "Containers" || section == "Conditions" { if section != "Events" { o.push_str(&format!("{}\n", l.trim_end())); } continue; }
            if SCALARS.iter().any(|s| l.starts_with(s)) { o.push_str(&format!("{}\n", trunc(&l.split_whitespace().collect::<Vec<_>>().join(" "), 120))); }
            continue;
        }
        let t = l.trim();
        match section {
            "Containers" if !l.starts_with("      ") && t.ends_with(':') => o.push_str(&format!("  {t}\n")),
            "Containers" if CONTAINER_KEYS.iter().any(|k| t.starts_with(k)) && l.starts_with("    ") => o.push_str(&format!("    {}\n", trunc(&t.split_whitespace().collect::<Vec<_>>().join(" "), 120))),
            "Conditions" if !t.starts_with("Type") && !t.starts_with("----") && !t.is_empty() => o.push_str(&format!("  {}\n", t.split_whitespace().collect::<Vec<_>>().join(" "))),
            "Events" if !t.starts_with("Type") && !t.starts_with("----") && !t.is_empty() && t != "<none>" => events.push(t),
            _ => {}
        }
    }
    if !events.is_empty() {
        o.push_str(&format!("Events ({}{}):\n", events.len(), if events.len() > 8 { ", last 8" } else { "" }));
        for e in events.iter().rev().take(8).rev() { o.push_str(&format!("  {}\n", trunc(&e.split_whitespace().collect::<Vec<_>>().join(" "), 160))); }
    }
    Some(o)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn l(s: &str) -> Vec<String> { s.lines().map(String::from).collect() }
    fn a(s: &str) -> Vec<String> { s.split_whitespace().map(String::from).collect() }
    #[test]
    fn get_pods_drops_columns() {
        let raw = "NAME                    READY   STATUS             RESTARTS   AGE   IP           NODE\nweb-1                   1/1     Running            0          3d    10.0.0.4     n1\nweb-2                   0/1     CrashLoopBackOff   12         3d    10.0.0.5     n2";
        let o = view(&a("get pods"), &l(raw)).unwrap();
        assert!(o.contains("web-2 0/1 CrashLoopBackOff 12") && !o.contains("10.0.0.4") && !o.contains("3d"));
    }
    #[test]
    fn get_many_pods_summarises_health() {
        let mut raw = String::from("NAME      READY   STATUS      RESTARTS   AGE\n");
        for i in 0..40 { raw.push_str(&format!("pod-{i:<5} 1/1     Running     0          1d\n")); }
        raw.push_str("bad-pod   0/1     Error       3          1d\n");
        let o = view(&a("get pods -A"), &l(&raw)).unwrap();
        assert!(o.contains("status: Running 40, Error 1") && o.contains("bad-pod") && !o.contains("pod-3 "));
    }
    #[test]
    fn describe_keeps_signal() {
        let raw = "Name:         web-1\nNamespace:    default\nPriority:     0\nNode:         n1/10.0.0.1\nLabels:       app=web\n              tier=front\nAnnotations:  a=b\nStatus:       Running\nContainers:\n  web:\n    Container ID:   docker://abc\n    Image:          nginx:1.25\n    Port:           80/TCP\n    State:          Running\n      Started:      Mon, 01 Jan 2024\n    Ready:          True\n    Restart Count:  0\nConditions:\n  Type              Status\n  Ready             True\nVolumes:\n  kube-api-access:\n    Type:  Projected\nQoS Class:       BestEffort\nTolerations:     node.kubernetes.io/not-ready:NoExecute op=Exists\nEvents:\n  Type     Reason   Age   From     Message\n  ----     ------   ----  ----     -------\n  Normal   Pulled   3d    kubelet  Image pulled\n  Warning  BackOff  1m    kubelet  Back-off restarting failed container";
        let o = view(&a("describe pod web-1"), &l(raw)).unwrap();
        assert!(o.contains("Name: web-1") && o.contains("Status: Running") && o.contains("Image: nginx:1.25") && o.contains("Ready True"));
        assert!(o.contains("Warning BackOff 1m kubelet Back-off") && !o.contains("Tolerations") && !o.contains("Annotations") && !o.contains("Container ID"));
        assert!(o.len() < raw.len());
    }
}
