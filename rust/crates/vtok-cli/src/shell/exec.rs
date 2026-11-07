//! Running the command: through `sh`, stderr merged into stdout, output capped at 64 MiB, optional timeout.
use std::io::Read;
use std::process::{Command, Stdio};

const MAX_CAPTURE: usize = 64 * 1024 * 1024;

pub struct Ran { pub out: Vec<u8>, pub code: i32, pub timed_out: bool }

/// Run `cmd` (one string = a shell snippet, several = argv) with stderr merged into stdout.
pub fn execute(cmd: &[String], cwd: Option<&str>, timeout: Option<std::time::Duration>, inherit_stdin: bool) -> Result<Ran, String> {
    let mut c = Command::new("sh");
    if cmd.len() == 1 { c.arg("-c").arg(format!("{{ {}\n}} 2>&1", cmd[0])); }
    else { c.arg("-c").arg("exec \"$@\" 2>&1").arg("sh").args(cmd); }
    if let Some(d) = cwd { c.current_dir(d); }
    crate::diag::set_base(cwd); // diagnostics views show the absolute paths the tool printed relative to here
    c.stdin(if inherit_stdin { Stdio::inherit() } else { Stdio::null() }).stdout(Stdio::piped()).stderr(Stdio::inherit());
    #[cfg(unix)]
    { use std::os::unix::process::CommandExt; c.process_group(0); }
    let mut child = c.spawn().map_err(|e| format!("cannot start: {e}"))?;
    let mut stdout = child.stdout.take().ok_or("no stdout")?;
    let reader = std::thread::spawn(move || {
        let mut buf = vec![];
        let _ = (&mut stdout).take(MAX_CAPTURE as u64).read_to_end(&mut buf);
        let _ = std::io::copy(&mut stdout, &mut std::io::sink());
        buf
    });
    let mut timed_out = false;
    let status = match timeout {
        None => child.wait().ok(),
        Some(t) => {
            let start = std::time::Instant::now();
            loop {
                match child.try_wait() {
                    Ok(Some(st)) => break Some(st),
                    Ok(None) if start.elapsed() >= t => {
                        timed_out = true;
                        let _ = Command::new("kill").args(["-KILL", "--", &format!("-{}", child.id())]).status();
                        let _ = child.kill();
                        break child.wait().ok();
                    }
                    Ok(None) => std::thread::sleep(std::time::Duration::from_millis(20)),
                    Err(_) => break None,
                }
            }
        }
    };
    let out = reader.join().unwrap_or_default();
    let code = status.map_or(1, |st| st.code().unwrap_or_else(|| {
        #[cfg(unix)] { use std::os::unix::process::ExitStatusExt; 128 + st.signal().unwrap_or(0) }
        #[cfg(not(unix))] { 1 }
    }));
    Ok(Ran { out, code, timed_out })
}
