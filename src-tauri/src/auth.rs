//! Drives the daemon's GUI login: `hyperspell login --json` streams the
//! device-auth flow as newline-delimited JSON, which we forward to the frontend
//! as `login-event` events. JSON mode is auth-only — it does NOT install
//! auto-start or write AI-tool configs, so the app keeps sole ownership of
//! supervision (supervisor.rs) and consent (permissions.rs).

use crate::daemon_paths::daemon_bin;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use tauri::{AppHandle, Emitter};

/// Spawn `hyperspell login --json --app-slug <slug> --name <name>` and forward
/// each JSONL event (`{"event":"pending"|"approved"|"error", ...}`) to the
/// frontend. Returns once the child is spawned; the outcome arrives via events.
pub fn start_login(app: AppHandle, app_slug: String, name: String) -> Result<(), String> {
    let mut child = Command::new(daemon_bin())
        .args(["login", "--json", "--app-slug", &app_slug, "--name", &name])
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|e| format!("failed to start login: {e}"))?;

    let stdout = child.stdout.take().ok_or("no stdout from login")?;
    std::thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines().map_while(Result::ok) {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&line) {
                let _ = app.emit("login-event", value);
            }
        }
        let _ = child.wait();
    });
    Ok(())
}
