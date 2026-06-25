//! Spawns and supervises the bundled `hyperspell sync --supervised` daemon. The
//! app is the *sole* supervisor: `--supervised` makes the daemon tear down any
//! legacy launchd/systemd/cron auto-start on startup so the two don't fight.

use crate::daemon_paths::daemon_bin;
use std::process::{Child, Command};
use std::sync::Mutex;

#[derive(Default)]
pub struct Supervisor(pub Mutex<Option<Child>>);

impl Supervisor {
    /// Start the daemon if it isn't already running under us.
    ///
    /// TODO(self-restart): the daemon `sys.exit(0)`s after a successful wheel
    /// self-upgrade, expecting its supervisor to respawn it on the new code
    /// (this is what launchd did). A real impl spawns a monitor thread that
    /// `wait()`s and restarts on exit-0 / backoff-restarts on crash. The
    /// scaffold just launches it once.
    pub fn start(&self) -> Result<(), String> {
        let mut guard = self.0.lock().map_err(|e| e.to_string())?;
        if guard.is_some() {
            return Ok(()); // already supervising
        }
        let child = Command::new(daemon_bin())
            .args(["sync", "--supervised"])
            .spawn()
            .map_err(|e| format!("failed to start daemon: {e}"))?;
        *guard = Some(child);
        Ok(())
    }

    /// Stop the supervised daemon (best-effort). Quitting the app = stop syncing.
    pub fn stop(&self) {
        if let Ok(mut guard) = self.0.lock() {
            if let Some(mut child) = guard.take() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }

    pub fn is_running(&self) -> bool {
        self.0.lock().map(|g| g.is_some()).unwrap_or(false)
    }
}
