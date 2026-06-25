//! Spawns and supervises the bundled `hyperspell sync --supervised` daemon. The
//! app is the *sole* supervisor: `--supervised` makes the daemon tear down any
//! legacy launchd/systemd/cron auto-start on startup so the two don't fight.
//!
//! A single monitor thread owns the supervision loop and replicates launchd:
//! - exit 0 → the daemon self-upgraded its wheel (`sys.exit(0)`); respawn now.
//! - nonzero → a crash; back off (launchd's ThrottleInterval=30s) then respawn.
//! - stop() → user quit the app; don't respawn (quitting = stop syncing).

use crate::daemon_paths::daemon_bin;
use std::process::{Child, Command};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Crash backoff, matching the daemon's launchd `ThrottleInterval` / systemd
/// `RestartSec`.
const CRASH_BACKOFF: Duration = Duration::from_secs(30);
/// How often the monitor checks whether the child has exited.
const POLL: Duration = Duration::from_secs(1);

#[derive(Default)]
struct Inner {
    /// Whether the user wants the daemon running. Cleared by stop().
    running: AtomicBool,
    /// Whether the monitor thread has been spawned (only ever one).
    monitor_started: AtomicBool,
    /// The currently-running daemon process, if any.
    child: Mutex<Option<Child>>,
}

#[derive(Default, Clone)]
pub struct Supervisor {
    inner: Arc<Inner>,
}

fn spawn_daemon() -> Result<Child, String> {
    Command::new(daemon_bin())
        .args(["sync", "--supervised"])
        .spawn()
        .map_err(|e| format!("failed to start daemon: {e}"))
}

impl Supervisor {
    /// Begin supervising the daemon. Idempotent: only one monitor thread is ever
    /// started; later calls just re-arm `running`.
    pub fn start(&self) -> Result<(), String> {
        self.inner.running.store(true, Ordering::SeqCst);

        // Spawn the first child eagerly so a failure (e.g. binary missing) is
        // reported synchronously to the caller rather than swallowed by the thread.
        {
            let mut guard = self.inner.child.lock().map_err(|e| e.to_string())?;
            if guard.is_none() {
                *guard = Some(spawn_daemon()?);
            }
        }

        if self.inner.monitor_started.swap(true, Ordering::SeqCst) {
            return Ok(()); // monitor already running
        }

        let inner = Arc::clone(&self.inner);
        std::thread::spawn(move || monitor(inner));
        Ok(())
    }

    /// Stop the supervised daemon and prevent respawn. Quitting the app = stop
    /// syncing.
    pub fn stop(&self) {
        self.inner.running.store(false, Ordering::SeqCst);
        if let Ok(mut guard) = self.inner.child.lock() {
            if let Some(mut child) = guard.take() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }

    pub fn is_running(&self) -> bool {
        self.inner
            .child
            .lock()
            .map(|g| g.is_some())
            .unwrap_or(false)
    }
}

/// The supervision loop. Runs on its own thread until stop() clears `running`.
fn monitor(inner: Arc<Inner>) {
    loop {
        if !inner.running.load(Ordering::SeqCst) {
            // Stop requested: make sure nothing is left running, then exit.
            if let Ok(mut guard) = inner.child.lock() {
                if let Some(mut child) = guard.take() {
                    let _ = child.kill();
                    let _ = child.wait();
                }
            }
            inner.monitor_started.store(false, Ordering::SeqCst);
            return;
        }

        // Ensure a child exists (start() spawns the first; we respawn the rest).
        {
            let mut guard = match inner.child.lock() {
                Ok(g) => g,
                Err(_) => return, // poisoned; bail rather than spin
            };
            if guard.is_none() {
                match spawn_daemon() {
                    Ok(c) => *guard = Some(c),
                    Err(_) => {
                        drop(guard);
                        std::thread::sleep(CRASH_BACKOFF);
                        continue;
                    }
                }
            }
        }

        std::thread::sleep(POLL);

        // Has the child exited? Decide respawn timing without holding the lock
        // across the backoff sleep.
        let exit_backoff = {
            let mut guard = match inner.child.lock() {
                Ok(g) => g,
                Err(_) => return,
            };
            match guard.as_mut().map(Child::try_wait) {
                Some(Ok(Some(status))) => {
                    *guard = None;
                    // exit 0 → self-upgrade restart: respawn now. crash → back off.
                    if status.success() {
                        None
                    } else {
                        Some(CRASH_BACKOFF)
                    }
                }
                _ => None, // still running, or already taken
            }
        };
        if let Some(backoff) = exit_backoff {
            std::thread::sleep(backoff);
        }
    }
}
