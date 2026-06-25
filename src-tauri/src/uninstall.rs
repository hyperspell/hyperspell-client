//! Remove Hyperspell from the machine — the reversibility half of the trust
//! story. Runs the daemon's own teardown (strips the agent-config managed blocks,
//! the world-readable mirror, and any legacy autostart), removes the PATH
//! symlinks + the shell-rc line we added, and (with `purge`) deletes
//! ~/.hyperspell entirely (config, venv, logs). App login-autostart is disabled
//! by the caller, which has the AppHandle.

use crate::daemon_paths::{daemon_bin, hyperspell_dir};
use std::path::Path;
use std::process::Command;

pub fn run(purge: bool) -> Result<(), String> {
    // 1. Daemon's own teardown (agent configs, mirror, legacy autostart).
    //    Best-effort — the daemon may already be gone.
    let mut args = vec!["uninstall"];
    if purge {
        args.push("--purge");
    }
    let _ = Command::new(daemon_bin()).args(&args).output();

    // 2. Remove the PATH symlinks we created, and strip our shell-rc line.
    if let Some(home) = dirs::home_dir() {
        let local_bin = home.join(".local").join("bin");
        for name in ["hyperspell", "uv"] {
            let _ = std::fs::remove_file(local_bin.join(name));
        }
        for rc in [".zshrc", ".bashrc"] {
            strip_marker_lines(&home.join(rc), "# hyperspell");
        }
    }

    // 3. Purge ~/.hyperspell entirely if asked (config, venv, logs, marker).
    if purge {
        let _ = std::fs::remove_dir_all(hyperspell_dir());
    }
    Ok(())
}

fn strip_marker_lines(path: &Path, marker: &str) {
    let Ok(text) = std::fs::read_to_string(path) else {
        return;
    };
    let kept: Vec<&str> = text.lines().filter(|l| !l.contains(marker)).collect();
    let _ = std::fs::write(path, format!("{}\n", kept.join("\n")));
}
